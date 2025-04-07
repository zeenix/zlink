pub(crate) mod listener;
mod method_stream;
mod select_all;
pub mod service;

use core::{future::Future, pin::Pin};

use futures_util::{FutureExt, Stream, StreamExt};
use mayheap::Vec;
use method_stream::MethodStream;
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
        let mut method_streams = Vec::<_, MAX_CONNECTIONS>::new();
        let mut writers = Vec::<_, MAX_CONNECTIONS>::new();
        let mut reply_streams =
            Vec::<ReplyStream<Service::ReplyStream, Listener::Socket>, MAX_CONNECTIONS>::new();
        let mut reader_modified = false;

        loop {
            if reader_modified {
                // If the connection was added or removed from the list, any or all the streams
                // could have been invalidated since the readers they're pointing to could have
                // been moved. So we need to reinitialize all the method streams.
                method_streams.clear();
                // SAFETY:
                //
                // The compiler is unable to determine that the mutable borrows of `readers` and
                // `method_streams` here is mutually exclusive to the other ones below. Hence we use
                // `unsafe` to cast the mutable references to raw pointers and then back to mutable
                // references.
                let readers = unsafe { &mut *(&mut readers as *mut Vec<_, MAX_CONNECTIONS>) };
                for reader in readers.iter_mut() {
                    let stream = MethodStream::new(
                        reader,
                        ReadConnection::receive_call::<Service::MethodCall<'_>>,
                    );
                    method_streams
                        .push(Box::pin(stream))
                        .map_err(|_| crate::Error::BufferOverflow)?;
                }
                reader_modified = false;
            }
            let mut reply_futures = SelectAll::new();
            for stream in reply_streams.iter_mut() {
                reply_futures
                    .push(stream.stream.next())
                    .map_err(|_| crate::Error::BufferOverflow)?;
            }

            #[derive(Debug)]
            enum Either<Sock: Socket, MethodCall, ReplyStreamItem> {
                NewConnection {
                    conn: Connection<Sock>,
                },
                Call {
                    idx: usize,
                    call: Option<crate::Result<Call<MethodCall>>>,
                },
                ReplyStreamReply {
                    idx: usize,
                    reply: Option<ReplyStreamItem>,
                },
            }

            let either = futures_util::select_biased! {
                // 1. Accept a new connection.
                conn = listener.accept().fuse() => Either::NewConnection { conn: conn? },
                // 2. Read method calls from the existing connections..
                res = self.get_next_call(&mut method_streams).fuse() => {
                    let res = res?;

                    Either::Call { idx: res.0, call: res.1 }
                }
                // 3. Read replies from the reply streams.
                reply = reply_futures.fuse() => {
                    let (idx, reply) = reply;

                    Either::ReplyStreamReply { idx, reply }
                },
            };

            match either {
                Either::NewConnection { conn } => {
                    let (read, write) = conn.split();
                    readers
                        .push(read)
                        .map_err(|_| crate::Error::BufferOverflow)?;
                    reader_modified = true;
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
                        Some(Ok(call)) => match self.handle_call(call, &mut writers[idx]).await {
                            Ok(None) => remove = false,
                            Ok(Some(s)) => stream = Some(s),
                            Err(e) => println!("Error writing to connection: {e:?}"),
                        },
                        Some(Err(e)) => println!("Error reading from socket: {e:?}"),
                        None => println!("Stream closed"),
                    }

                    if stream.is_some() || remove {
                        let reader = readers.remove(idx);
                        let writer = writers.remove(idx);
                        reader_modified = true;

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
    async fn get_next_call<'r, F, Fut>(
        &mut self,
        method_streams: &mut MethodStreams<'r, <Listener::Socket as Socket>::ReadHalf, F, Fut>,
    ) -> crate::Result<(usize, Option<crate::Result<Call<Service::MethodCall<'r>>>>)>
    where
        F: FnMut(&'r mut ReadConnection<<Listener::Socket as Socket>::ReadHalf>) -> Fut,
        Fut: Future<Output = crate::Result<Call<Service::MethodCall<'r>>>>,
    {
        let mut read_futures = SelectAll::new();
        for stream in method_streams.iter_mut() {
            read_futures
                .push(stream.next())
                .map_err(|_| crate::Error::BufferOverflow)?;
        }

        Ok(read_futures.await)
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

type MethodStreams<'c, Read, F, Fut> =
    Vec<Pin<Box<MethodStream<'c, Read, F, Fut>>>, MAX_CONNECTIONS>;

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
