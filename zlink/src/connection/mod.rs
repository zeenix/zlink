//! Contains connection related API.

mod read_connection;
pub use read_connection::ReadConnection;
pub mod socket;
mod write_connection;
use core::{fmt::Debug, sync::atomic::AtomicUsize};
pub use write_connection::WriteConnection;
mod call;
pub use call::Call;
mod reply;
pub use reply::Reply;

use serde::{Deserialize, Serialize};
pub use socket::Socket;

/// A connection.
///
/// The low-level API to send and receive messages.
///
/// Each connection gets a unique identifier when created that can be queried using
/// [`Connection::id`]. This ID is shared betwen the read and write halves of the connection. It
/// can be used to associate the read and write halves of the same connection.
///
/// # Cancel safety
///
/// All async methods of this type are cancel safe unless explicitly stated otherwise in its
/// documentation.
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
        let id = NEXT_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        Self {
            read: ReadConnection::new(read, id),
            write: WriteConnection::new(write, id),
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

    /// The unique identifier of the connection.
    pub fn id(&self) -> usize {
        assert_eq!(self.read.id(), self.write.id());
        self.read.id()
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
#[cfg(all(
    not(feature = "io-buffer-1mb"),
    not(feature = "io-buffer-16kb"),
    not(feature = "io-buffer-4kb"),
    feature = "io-buffer-2kb"
))]
pub(crate) const BUFFER_SIZE: usize = 4 * 1024;

#[cfg(feature = "std")]
const MAX_BUFFER_SIZE: usize = 100 * 1024 * 1024; // Don't allow buffers over 100MB.

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
