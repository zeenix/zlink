use crate::{Connection, Result};

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
