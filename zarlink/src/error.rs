/// The Error type for the zarlink crate.
#[derive(Debug)]
pub enum Error {
    /// An error occurred while reading from the socket.
    SocketRead,
    /// An error occurred while writing to the socket.
    SocketWrite,
}

/// The Result type for the zarlink crate.
pub type Result<T> = core::result::Result<T, Error>;
