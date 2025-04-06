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

use crate::connection::{socket::WriteHalf, Call, ReadConnection, Socket, WriteConnection};

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
        let mut reply_streams = Vec::<
            ReplyStream<Service::ReplyStream, <Listener::Socket as Socket>::WriteHalf>,
            MAX_CONNECTIONS,
        >::new();

        loop {
            let mut reply_futures = SelectAll::new();
            for stream in reply_streams.iter_mut() {
                reply_futures
                    .push(stream.stream.next())
                    .map_err(|_| crate::Error::BufferOverflow)?;
            }

            futures_util::select_biased! {
                // Accept a new connection.
                conn = listener.accept().fuse() => {
                    let (read, write) = conn?.split();
                    let readers = unsafe { &mut *(&mut readers as *mut Vec<_, MAX_CONNECTIONS>) };
                    readers
                        .push(read)
                        .map_err(|_| crate::Error::BufferOverflow)?;
                    let stream = MethodStream::new(
                        readers.last_mut().unwrap(),
                        ReadConnection::receive_call::<Service::MethodCall<'_>>,
                    );
                    method_streams
                        .push(Box::pin(stream))
                        .map_err(|_| crate::Error::BufferOverflow)?;
                    writers
                        .push(write)
                        .map_err(|_| crate::Error::BufferOverflow)?;
                },
                res = self.handle_next(
                    // SAFETY:
                    //
                    // The compiler is unable to determine that the mutable borrow of
                    // `method_streams` in the other arm is mutually exclusive with the one here.
                    unsafe { &mut *(&mut method_streams as *mut _) },
                    unsafe { &mut *(&mut readers as *mut _) },
                    &mut writers,
                ).fuse() => if let Some(stream) = res? {
                    reply_streams
                        .push(stream)
                        .map_err(|_| crate::Error::BufferOverflow)?
                },
                reply = reply_futures.fuse() => {
                    let (idx, reply) = reply;
                    match reply {
                        Some(reply) => {
                            if let Err(e) = reply_streams
                            .get_mut(idx)
                            .unwrap()
                            .conn
                            .send_reply(Some(reply), Some(true)).await {
                                println!("Error writing to connection: {e:?}");
                                reply_streams.remove(idx);
                            }
                        }
                        None => {
                            println!("Stream closed");
                            reply_streams.remove(idx);
                        }
                    }
                },
            }
        }
    }

    /// Read the next method call from the connection and handle it.
    async fn handle_next<'r, F, Fut>(
        &mut self,
        method_streams: &mut MethodStreams<'r, <Listener::Socket as Socket>::ReadHalf, F, Fut>,
        readers: &'r mut Vec<
            ReadConnection<<Listener::Socket as Socket>::ReadHalf>,
            MAX_CONNECTIONS,
        >,
        writers: &mut Vec<
            WriteConnection<<Listener::Socket as Socket>::WriteHalf>,
            MAX_CONNECTIONS,
        >,
    ) -> crate::Result<
        Option<ReplyStream<Service::ReplyStream, <Listener::Socket as Socket>::WriteHalf>>,
    >
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

        let (idx, call) = read_futures.await;
        let mut stream = None;
        match call {
            Some(Ok(call)) => match self.service.handle(call).await {
                Reply::Single(reply) => match writers[idx].send_reply(reply, Some(false)).await {
                    Ok(_) => return Ok(None),
                    Err(e) => println!("Error writing to connection: {e:?}"),
                },
                Reply::Error(err) => match writers[idx].send_error(err).await {
                    Ok(_) => return Ok(None),
                    Err(e) => println!("Error writing to connection: {e:?}"),
                },
                Reply::Multi(s) => stream = Some(s),
            },
            Some(Err(e)) => println!("Error reading from socket: {e:?}"),
            None => println!("Stream closed"),
        }

        // If we reach here, the stream was closed or an error occurred or we're going to stream the
        // reply, in which case the connection now only exists for the stream.
        method_streams.remove(idx);
        readers.remove(idx);
        let writer = writers.remove(idx);

        Ok(stream.map(|s| ReplyStream::new(s, writer)))
    }
}

const MAX_CONNECTIONS: usize = 16;

type MethodStreams<'c, Read, F, Fut> =
    Vec<Pin<Box<MethodStream<'c, Read, F, Fut>>>, MAX_CONNECTIONS>;

/// Method reply stream and connection pair.
#[derive(Debug)]
struct ReplyStream<St, Write: WriteHalf> {
    stream: Pin<Box<St>>,
    conn: WriteConnection<Write>,
}

impl<St, Write> ReplyStream<St, Write>
where
    St: Stream,
    <St as Stream>::Item: Serialize + core::fmt::Debug,
    Write: WriteHalf,
{
    fn new(stream: St, conn: WriteConnection<Write>) -> Self {
        Self {
            stream: Box::pin(stream),
            conn,
        }
    }
}
