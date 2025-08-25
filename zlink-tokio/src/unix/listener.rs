use std::os::fd::OwnedFd;

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

impl TryFrom<OwnedFd> for Listener {
    type Error = crate::Error;

    fn try_from(fd: OwnedFd) -> Result<Self> {
        let std_listener = std::os::unix::net::UnixListener::from(fd);
        std_listener.set_nonblocking(true)?;

        tokio::net::UnixListener::from_std(std_listener)
            .map(|listener| Listener { listener })
            .map_err(Into::into)
    }
}
