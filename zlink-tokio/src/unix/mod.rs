//! Provides transport over Unix Domain Sockets.

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};
use zlink::{connection::Socket, Result};

/// The connection type that uses Unix Domain Sockets for transport.
pub type Connection = zlink::Connection<Stream>;

/// Connect to Unix Domain Socket at the given path.
pub async fn connect<P>(path: P) -> Result<Connection>
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
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.0.read(buf).await.map_err(Into::into)
    }

    async fn write(&mut self, buf: &[u8]) -> Result<()> {
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
