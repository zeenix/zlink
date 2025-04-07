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

            // SAFETY:
            //
            // The compiler is unable to determine that the mutable borrows of `readers` and
            // `method_streams` in one arm are mutually exclusive to the other ones. Hence we use
            // `unsafe` to cast the mutable references to raw pointers and then back to mutable
            // references.
            futures_util::select_biased! {
                // Accept a new connection.
                conn = listener.accept().fuse() => {
                    let (read, write) = conn?.split();
                    let readers = unsafe { &mut *(&mut readers as *mut Vec<_, MAX_CONNECTIONS>) };
                    readers
                        .push(read)
                        .map_err(|_| crate::Error::BufferOverflow)?;
                    reader_modified = true;
                    writers
                        .push(write)
                        .map_err(|_| crate::Error::BufferOverflow)?;
                },
                res = self.handle_next(
                    unsafe { &mut *(&mut method_streams as *mut _) },
                    unsafe { &mut *(&mut readers as *mut _) },
                    &mut writers,
                ).fuse() => {
                    let (modified, stream) = res?;

                    if let Some(stream) = stream {
                        reply_streams
                            .push(stream)
                            .map_err(|_| crate::Error::BufferOverflow)?;
                    }

                    reader_modified = modified;
                },
                reply = reply_futures.fuse() => {
                    let (idx, reply) = reply;
                    match reply {
                        Some(reply) => {
                            if let Err(e) = reply_streams
                            .get_mut(idx)
                            .unwrap()
                            .conn
                            .write_mut()
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
    ///
    /// # Caveats
    ///
    /// While this method removes the appropriate elements from all the three vectors on I/O errors
    /// or if the service method handler returns a streaming reply, it doesn't take into account the
    /// relationship between the `method_streams` and `readers` and therefore does **not**
    /// reinitialize the `method_streams` elements that may be invalidated by the removal of
    /// `readers` elements. The caller is responsible for reinitializing all the `method_streams`
    /// elements if this method returns `true`.
    ///
    /// # Return value
    ///
    /// On success, this method returns a tuple containing:
    ///
    /// * boolean indicating if the `readers` was modified.
    /// * an optional reply stream if the method call was a streaming method.
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
    ) -> crate::Result<(
        bool,
        Option<ReplyStream<Service::ReplyStream, Listener::Socket>>,
    )>
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
            Some(Ok(call)) => match self.handle_call(call, &mut writers[idx]).await {
                Ok(None) => return Ok((false, None)),
                Ok(Some(s)) => stream = Some(s),
                Err(e) => println!("Error writing to connection: {e:?}"),
            },
            Some(Err(e)) => println!("Error reading from socket: {e:?}"),
            None => println!("Stream closed"),
        }

        // If we reach here, the stream was closed or an error occurred or we're going to stream the
        // reply, in which case the connection now only exists for the stream.
        let reader = readers.remove(idx);
        let writer = writers.remove(idx);

        Ok((true, stream.map(|s| ReplyStream::new(s, reader, writer))))
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
