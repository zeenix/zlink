use core::future::Future;

use crate::{connection::Socket, Connection, Result};

/// A listener is a server that listens for incoming connections.
pub trait Listener: core::fmt::Debug {
    /// The type of the socket the connections this listener creates will use.
    type Socket: Socket;

    /// Accept a new connection.
    fn accept(&mut self) -> impl Future<Output = Result<Connection<Self::Socket>>>;
}
