/// The Error type for the zarlink crate.
#[derive(Debug)]
pub enum Error {
    /// An error occurred while reading from the socket.
    SocketRead,
    /// An error occurred while writing to the socket.
    SocketWrite,
    /// Buffer overflow.
    BufferOverflow,
    /// Error serializing or deserializing to/from JSON.
    #[cfg(feature = "std")]
    Json(serde_json::Error),
    /// Error serialization to JSON.
    #[cfg(not(feature = "std"))]
    JsonSerialize(serde_json_core::ser::Error),
}

/// The Result type for the zarlink crate.
pub type Result<T> = core::result::Result<T, Error>;

#[cfg(feature = "std")]
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Json(e)
    }
}

#[cfg(not(feature = "std"))]
impl From<serde_json_core::ser::Error> for Error {
    fn from(e: serde_json_core::ser::Error) -> Self {
        Error::JsonSerialize(e)
    }
}
