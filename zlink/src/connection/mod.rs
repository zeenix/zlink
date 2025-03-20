//! Contains connection related API.

mod read_connection;
pub use read_connection::ReadConnection;
pub mod socket;
mod write_connection;
use core::fmt::Debug;
pub use write_connection::WriteConnection;

use serde::{Deserialize, Serialize};
pub use socket::Socket;

/// A connection.
///
/// The low-level API to send and receive messages.
#[derive(Debug)]
pub struct Connection<S: Socket> {
    read: ReadConnection<S::ReadHalf>,
    write: WriteConnection<S::WriteHalf>,
}

impl<S> Connection<S>
where
    S: Socket,
{
    /// Create a new connection.
    pub fn new(socket: S) -> Self {
        let (read, write) = socket.split();
        Self {
            read: ReadConnection::new(read),
            write: WriteConnection::new(write),
        }
    }

    /// The mutable reference to the read half of the connection.
    pub fn read(&mut self) -> &mut ReadConnection<S::ReadHalf> {
        &mut self.read
    }

    /// The mutable reference to the write half of the connection.
    pub fn write(&mut self) -> &mut WriteConnection<S::WriteHalf> {
        &mut self.write
    }

    /// Split the connection into read and write halves.
    pub fn split(self) -> (ReadConnection<S::ReadHalf>, WriteConnection<S::WriteHalf>) {
        (self.read, self.write)
    }

    /// Sends a method call.
    ///
    /// Convenience wrapper around [`WriteConnection::send_call`].
    pub async fn send_call<Method>(
        &mut self,
        method: Method,
        oneway: Option<bool>,
        more: Option<bool>,
        upgrade: Option<bool>,
    ) -> crate::Result<()>
    where
        Method: Serialize + Debug,
    {
        self.write.send_call(method, oneway, more, upgrade).await
    }

    /// Receives a method call reply.
    ///
    /// Convenience wrapper around [`ReadConnection::receive_reply`].
    pub async fn receive_reply<'r, Params, ReplyError>(
        &'r mut self,
    ) -> crate::Result<Result<Reply<Params>, ReplyError>>
    where
        Params: Deserialize<'r>,
        ReplyError: Deserialize<'r>,
    {
        self.read.receive_reply().await
    }

    /// Call a method and receive a reply.
    ///
    /// This is a convenience method that combines [`Connection::send_call`] and
    /// [`Connection::receive_reply`].
    pub async fn call_method<'r, Method, ReplyError, Params>(
        &'r mut self,
        method: Method,
        oneway: Option<bool>,
        more: Option<bool>,
        upgrade: Option<bool>,
    ) -> crate::Result<Result<Reply<Params>, ReplyError>>
    where
        Method: Serialize + Debug,
        Params: Deserialize<'r>,
        ReplyError: Deserialize<'r>,
    {
        self.send_call(method, oneway, more, upgrade).await?;
        self.receive_reply().await
    }

    /// Receive a method call over the socket.
    ///
    /// Convenience wrapper around [`ReadConnection::receive_call`].
    pub async fn receive_call<'m, Method>(&'m mut self) -> crate::Result<Call<Method>>
    where
        Method: Deserialize<'m>,
    {
        self.read.receive_call().await
    }

    /// Send a reply over the socket.
    ///
    /// Convenience wrapper around [`WriteConnection::send_reply`].
    pub async fn send_reply<Params>(
        &mut self,
        parameters: Option<Params>,
        continues: Option<bool>,
    ) -> crate::Result<()>
    where
        Params: Serialize + Debug,
    {
        self.write.send_reply(parameters, continues).await
    }

    /// Send an error reply over the socket.
    ///
    /// Convenience wrapper around [`WriteConnection::send_error`].
    pub async fn send_error<ReplyError>(&mut self, error: ReplyError) -> crate::Result<()>
    where
        ReplyError: Serialize + Debug,
    {
        self.write.send_error(error).await
    }
}

/// A successful method call reply.
#[derive(Debug, Serialize, Deserialize)]
pub struct Reply<Params> {
    #[serde(skip_serializing_if = "Option::is_none")]
    parameters: Option<Params>,
    #[serde(skip_serializing_if = "Option::is_none")]
    continues: Option<bool>,
}

impl<Params> Reply<Params> {
    /// The parameters of the reply.
    pub fn parameters(&self) -> Option<&Params> {
        self.parameters.as_ref()
    }

    /// Convert the reply into its parameters.
    pub fn into_parameters(self) -> Option<Params> {
        self.parameters
    }

    /// If there are more replies to come.
    pub fn continues(&self) -> Option<bool> {
        self.continues
    }
}

/// A method call.
#[derive(Debug, Serialize, Deserialize)]
pub struct Call<M> {
    #[serde(flatten)]
    method: M,
    #[serde(skip_serializing_if = "Option::is_none")]
    oneway: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    more: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    upgrade: Option<bool>,
}

impl<M> Call<M> {
    /// The method call name and parameters.
    pub fn method(&self) -> &M {
        &self.method
    }

    /// If the method call doesn't want a reply.
    pub fn oneway(&self) -> Option<bool> {
        self.oneway
    }

    /// If the method call is requesting more replies.
    pub fn more(&self) -> Option<bool> {
        self.more
    }

    /// If the method call is requesting an upgrade to a different protocol.
    pub fn upgrade(&self) -> Option<bool> {
        self.upgrade
    }
}

#[cfg(feature = "io-buffer-1mb")]
pub(crate) const BUFFER_SIZE: usize = 1024 * 1024;
#[cfg(all(not(feature = "io-buffer-1mb"), feature = "io-buffer-16kb"))]
pub(crate) const BUFFER_SIZE: usize = 16 * 1024;
#[cfg(all(
    not(feature = "io-buffer-1mb"),
    not(feature = "io-buffer-16kb"),
    feature = "io-buffer-4kb"
))]
pub(crate) const BUFFER_SIZE: usize = 4 * 1024;

#[cfg(feature = "std")]
const MAX_BUFFER_SIZE: usize = 100 * 1024 * 1024; // Don't allow buffers over 100MB.
