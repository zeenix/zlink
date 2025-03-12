//! Provides transport over Unix Domain Sockets.

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};
use zarlink::{connection::Socket, Result};

/// The connection type that uses Unix Domain Sockets for transport.
pub type Connection = zarlink::Connection<Stream>;

/// Connect to Unix Domain Socket at the given path.
pub async fn connect<P>(path: P) -> Result<Connection, &'static str>
where
    P: AsRef<std::path::Path>,
{
    UnixStream::connect(path)
        .await
        .map(Stream)
        .map(Connection::new)
        .map_err(Into::into)
}

/// The [`Socket`] implementation using Unix Domain Sockets.
#[derive(Debug)]
pub struct Stream(UnixStream);

impl Socket for Stream {
    async fn read<ReplyError>(&mut self, buf: &mut [u8]) -> Result<usize, ReplyError> {
        self.0.read(buf).await.map_err(Into::into)
    }

    async fn write<ReplyError>(&mut self, buf: &[u8]) -> Result<(), ReplyError> {
        let mut pos = 0;

        while pos < buf.len() {
            let n = self.0.write(&buf[pos..]).await?;
            pos += n;
        }

        Ok(())
    }
}

impl From<UnixStream> for Stream {
    fn from(stream: UnixStream) -> Self {
        Self(stream)
    }
}
