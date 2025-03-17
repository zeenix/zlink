pub(crate) mod listener;
use futures_util::{pin_mut, StreamExt};
pub mod service;

/// A server.
///
/// The server listens for incoming connections and handles method calls using a service.
#[derive(Debug)]
pub struct Server<Listener, Service> {
    listener: Listener,
    service: Service,
}

impl<Listener, Service> Server<Listener, Service>
where
    Listener: listener::Listener,
    Service: service::Service,
{
    /// Create a new server.
    pub fn new(listener: Listener, service: Service) -> Self {
        Self { listener, service }
    }

    /// TODO:
    pub async fn run(mut self) -> crate::Result<()> {
        let mut connection = self.listener.accept().await?;
        loop {
            match self.service.handle_next(&mut connection).await {
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
