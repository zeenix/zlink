//! Provides transport over Unix Domain Sockets.

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};

/// The connection type that uses Unix Domain Sockets for transport.
pub type Connection = zarlink::Connection<Stream>;

/// The [`zarlink::connection::Socket`] implementation using Unix Domain Sockets.
#[derive(Debug)]
pub struct Stream(UnixStream);

impl Stream {
    /// Connect to Unix Domain Socket at the given path.
    pub async fn connect<P>(path: P) -> zarlink::Result<Connection>
    where
        P: AsRef<std::path::Path>,
    {
        UnixStream::connect(path)
            .await
            .map(Self)
            .map(Connection::new)
            .map_err(Into::into)
    }
}

impl zarlink::connection::Socket for Stream {
    async fn read(&mut self, buf: &mut [u8]) -> zarlink::Result<usize> {
        self.0.read(buf).await.map_err(Into::into)
    }

    async fn write(&mut self, buf: &[u8]) -> zarlink::Result<()> {
        let mut pos = 0;

        while pos < buf.len() {
            let n = self.0.write(&buf[pos..]).await?;
            pos += n;
        }

        Ok(())
    }
}
