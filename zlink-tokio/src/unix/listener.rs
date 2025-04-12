use crate::{Connection, Result};

/// Create a new unix domain socket listener and bind it to `path`.
pub fn bind<P>(path: P) -> Result<Listener>
where
    P: AsRef<std::path::Path>,
{
    tokio::net::UnixListener::bind(path)
        .map(|listener| Listener { listener })
        .map_err(Into::into)
}

/// A unix domain socket listener.
#[derive(Debug)]
pub struct Listener {
    listener: tokio::net::UnixListener,
}

impl crate::Listener for Listener {
    type Socket = super::Stream;

    async fn accept(&mut self) -> Result<Connection<Self::Socket>> {
        self.listener
            .accept()
            .await
            .map(|(stream, _)| super::Stream::from(stream).into())
            .map_err(Into::into)
    }
}
