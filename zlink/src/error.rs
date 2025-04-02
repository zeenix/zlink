/// The Error type for the zlink crate.
#[derive(Debug)]
#[non_exhaustive]
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
    /// Error deserialization from JSON.
    #[cfg(not(feature = "std"))]
    JsonDeserialize(serde_json_core::de::Error),
    /// An I/O error.
    #[cfg(feature = "std")]
    Io(std::io::Error),
}

/// The Result type for the zlink crate.
pub type Result<T> = core::result::Result<T, Error>;

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            #[cfg(feature = "std")]
            Error::Json(e) => Some(e),
            #[cfg(not(feature = "std"))]
            Error::JsonSerialize(e) => Some(e),
            #[cfg(not(feature = "std"))]
            Error::JsonDeserialize(e) => Some(e),
            #[cfg(feature = "std")]
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

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

#[cfg(not(feature = "std"))]
impl From<serde_json_core::de::Error> for Error {
    fn from(e: serde_json_core::de::Error) -> Self {
        Error::JsonDeserialize(e)
    }
}

#[cfg(feature = "std")]
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::SocketRead => write!(f, "An error occurred while reading from the socket"),
            Error::SocketWrite => write!(f, "An error occurred while writing to the socket"),
            Error::BufferOverflow => write!(f, "Buffer overflow"),
            #[cfg(feature = "std")]
            Error::Json(e) => write!(f, "Error serializing or deserializing to/from JSON: {e}"),
            #[cfg(not(feature = "std"))]
            Error::JsonSerialize(e) => write!(f, "Error serializing to JSON: {e}"),
            #[cfg(not(feature = "std"))]
            Error::JsonDeserialize(e) => write!(f, "Error deserializing from JSON: {e}"),
            #[cfg(feature = "std")]
            Error::Io(e) => write!(f, "I/O error: {e}"),
        }
    }
}
