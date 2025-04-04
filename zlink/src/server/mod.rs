pub(crate) mod listener;
mod method_stream;
mod select_all;
pub mod service;

use core::{future::Future, pin::Pin};

use futures_util::{pin_mut, FutureExt, StreamExt};
use mayheap::Vec;
use method_stream::MethodStream;
use service::Reply;

use crate::connection::{Call, ReadConnection, Socket, WriteConnection};

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

        loop {
            futures_util::select_biased! {
                // Accept a new connection.
                conn = listener.accept().fuse() => {
                    let (read, write) = conn?.split();
                    let stream = MethodStream::new(
                        read,
                        ReadConnection::receive_call::<Service::MethodCall<'_>>,
                    );
                    readers
                        .push(Box::pin(stream))
                        .map_err(|_| crate::Error::BufferOverflow)?;
                    writers
                        .push(write)
                        .map_err(|_| crate::Error::BufferOverflow)?;
                },
                res = self.handle_next(
                    // SAFETY:
                    //
                    // The compiler is unable to determine that the mutable borrow of `readers` in
                    // the other arm is mutually exclusive with the one here.
                    unsafe { &mut *(&mut readers as *mut _) },
                    &mut writers,
                ).fuse() => {
                    match res {
                        Ok(Some(stream)) => {
                            pin_mut!(stream);
                            while let Some(r) = stream.next().await {
                                println!("Streamed reply: {:?}", r);
                            }
                        }
                        Ok(None) => (),
                        Err(e) => {
                            // TODO:
                            println!("Error handling call: {e:?}");
                        }
                    }
                }
            }
        }
    }

    /// Read the next method call from the connection and handle it.
    async fn handle_next<'r, F, Fut>(
        &mut self,
        readers: &'r mut MethodStreams<<Listener::Socket as Socket>::ReadHalf, F, Fut>,
        writers: &mut Vec<
            WriteConnection<<Listener::Socket as Socket>::WriteHalf>,
            MAX_CONNECTIONS,
        >,
    ) -> crate::Result<Option<Service::ReplyStream>>
    where
        F: FnMut(&'r mut ReadConnection<<Listener::Socket as Socket>::ReadHalf>) -> Fut,
        Fut: Future<Output = crate::Result<Call<Service::MethodCall<'r>>>>,
    {
        let mut read_futures = select_all::SelectAll::new();
        for stream in readers.iter_mut() {
            read_futures
                .push(stream.next())
                .map_err(|_| crate::Error::BufferOverflow)?;
        }

        let (idx, call) = read_futures.await;
        match call {
            Some(Ok(call)) => match self.service.handle(call).await {
                Reply::Single(reply) => {
                    match writers
                        .get_mut(idx)
                        .unwrap()
                        .send_reply(reply, Some(false))
                        .await
                    {
                        Ok(_) => return Ok(None),
                        Err(e) => println!("Error writing to connection: {e:?}"),
                    }
                }
                Reply::Error(err) => match writers.get_mut(idx).unwrap().send_error(err).await {
                    Ok(_) => return Ok(None),
                    Err(e) => println!("Error writing to connection: {e:?}"),
                },
                Reply::Multi(stream) => return Ok(Some(stream)),
            },
            Some(Err(e)) => println!("Error reading from socket: {e:?}"),
            None => println!("Stream closed"),
        }

        // If we reach here, the stream was closed or an error occurred.
        readers.remove(idx);
        writers.remove(idx);

        Ok(None)
    }
}

const MAX_CONNECTIONS: usize = 16;

type MethodStreams<Read, F, Fut> = Vec<Pin<Box<MethodStream<Read, F, Fut>>>, MAX_CONNECTIONS>;
