pub(crate) mod listener;
mod select_all;
pub mod service;

use core::pin::Pin;

use futures_util::{FutureExt, Stream, StreamExt};
use mayheap::Vec;
use select_all::SelectAll;
use serde::Serialize;
use service::Reply;

use crate::{
    connection::{Call, ReadConnection, Socket, WriteConnection},
    Connection,
};

/// A server.
///
/// The server listens for incoming connections and handles method calls using a service.
#[derive(Debug)]
pub struct Server<Listener, Service> {
    listener: Option<Listener>,
    service: Service,
}

impl<Listener, Service> Server<Listener, Service>
where
    Listener: listener::Listener,
    Service: service::Service,
{
    /// Create a new server.
    pub fn new(listener: Listener, service: Service) -> Self {
        Self {
            listener: Some(listener),
            service,
        }
    }

    /// TODO:
    pub async fn run(mut self) -> crate::Result<()> {
        let mut listener = self.listener.take().unwrap();
        let mut readers = Vec::<_, MAX_CONNECTIONS>::new();
        let mut writers = Vec::<_, MAX_CONNECTIONS>::new();
        let mut reply_streams =
            Vec::<ReplyStream<Service::ReplyStream, Listener::Socket>, MAX_CONNECTIONS>::new();

        loop {
            #[derive(Debug)]
            enum Either<Sock: Socket, MethodCall, ReplyStreamItem> {
                NewConnection {
                    conn: Connection<Sock>,
                },
                Call {
                    idx: usize,
                    call: crate::Result<Call<MethodCall>>,
                },
                ReplyStreamReply {
                    idx: usize,
                    reply: Option<ReplyStreamItem>,
                },
            }

            let either = {
                let mut reply_stream_futures: Vec<_, MAX_CONNECTIONS> =
                    reply_streams.iter_mut().map(|s| s.stream.next()).collect();
                let mut reply_stream_select_all = SelectAll::new();
                for future in reply_stream_futures.iter_mut() {
                    reply_stream_select_all
                        .push(future)
                        .map_err(|_| crate::Error::BufferOverflow)?;
                }

                futures_util::select_biased! {
                    // 1. Accept a new connection.
                    conn = listener.accept().fuse() => Either::NewConnection { conn: conn? },
                    // 2. Read method calls from the existing connections..
                    res = self.get_next_call(
                        // SAFETY: `readers` is not invalidated or dropped until the output of this
                        // future is dropped.
                        unsafe { &mut *(&mut readers as *mut _) },
                    ).fuse() => {
                        let res = res?;

                        Either::Call { idx: res.0, call: res.1 }
                    }
                    // 3. Read replies from the reply streams.
                    reply = reply_stream_select_all.fuse() => {
                        let (idx, reply) = reply;

                        Either::ReplyStreamReply { idx, reply }
                    },
                }
            };

            match either {
                Either::NewConnection { conn } => {
                    let (read, write) = conn.split();
                    readers
                        .push(read)
                        .map_err(|_| crate::Error::BufferOverflow)?;
                    writers
                        .push(write)
                        .map_err(|_| crate::Error::BufferOverflow)?;
                }
                Either::ReplyStreamReply { idx, reply } => match reply {
                    Some(reply) => {
                        if let Err(e) = reply_streams
                            .get_mut(idx)
                            .unwrap()
                            .conn
                            .write_mut()
                            .send_reply(Some(reply), Some(true))
                            .await
                        {
                            println!("Error writing to connection: {e:?}");
                            reply_streams.remove(idx);
                        }
                    }
                    None => {
                        println!("Stream closed");
                        reply_streams.remove(idx);
                    }
                },
                Either::Call { idx, call } => {
                    let mut stream = None;
                    let mut remove = true;
                    match call {
                        Ok(call) => match self.handle_call(call, &mut writers[idx]).await {
                            Ok(None) => remove = false,
                            Ok(Some(s)) => stream = Some(s),
                            Err(e) => println!("Error writing to connection: {e:?}"),
                        },
                        Err(e) => println!("Error reading from socket: {e:?}"),
                    }

                    if stream.is_some() || remove {
                        let reader = readers.remove(idx);
                        let writer = writers.remove(idx);

                        if let Some(stream) = stream.map(|s| ReplyStream::new(s, reader, writer)) {
                            reply_streams
                                .push(stream)
                                .map_err(|_| crate::Error::BufferOverflow)?;
                        }
                    }
                }
            }
        }
    }

    /// Read the next method call from the connection.
    ///
    ///
    /// # Return value
    ///
    /// On success, this method returns a tuple containing:
    ///
    /// * boolean indicating if the `readers` was modified.
    /// * an optional reply stream if the method call was a streaming method.
    async fn get_next_call<'r>(
        &mut self,
        readers: &'r mut Vec<
            ReadConnection<<<Listener as crate::Listener>::Socket as Socket>::ReadHalf>,
            16,
        >,
    ) -> crate::Result<(usize, crate::Result<Call<Service::MethodCall<'r>>>)> {
        let mut read_futures: Vec<_, 16> = readers.iter_mut().map(|r| r.receive_call()).collect();
        let mut select_all = SelectAll::new();
        for future in &mut read_futures {
            // Safety: `future` is in fact `Unpin` but the compiler doesn't know that.
            unsafe {
                select_all
                    .push_unchecked(future)
                    .map_err(|_| crate::Error::BufferOverflow)?;
            }
        }

        Ok(select_all.await)
    }

    async fn handle_call(
        &mut self,
        call: Call<Service::MethodCall<'_>>,
        writer: &mut WriteConnection<<Listener::Socket as Socket>::WriteHalf>,
    ) -> crate::Result<Option<Service::ReplyStream>> {
        let mut stream = None;
        match self.service.handle(call).await {
            Reply::Single(reply) => writer.send_reply(reply, Some(false)).await?,
            Reply::Error(err) => writer.send_error(err).await?,
            Reply::Multi(s) => stream = Some(s),
        }

        Ok(stream)
    }
}

const MAX_CONNECTIONS: usize = 16;

/// Method reply stream and connection pair.
#[derive(Debug)]
struct ReplyStream<St, Sock: Socket> {
    stream: Pin<Box<St>>,
    conn: Connection<Sock>,
}

impl<St, Sock> ReplyStream<St, Sock>
where
    St: Stream,
    <St as Stream>::Item: Serialize + core::fmt::Debug,
    Sock: Socket,
{
    fn new(
        stream: St,
        read_conn: ReadConnection<Sock::ReadHalf>,
        write_conn: WriteConnection<Sock::WriteHalf>,
    ) -> Self {
        Self {
            stream: Box::pin(stream),
            conn: Connection::join(read_conn, write_conn),
        }
    }
}
