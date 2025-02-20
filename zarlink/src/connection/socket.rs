use core::future::Future;

/// The socket trait.
///
/// This is the trait that needs to be implemented for a type to be used as a socket/transport.
pub trait Socket {
    /// The error type returned by the methods.
    type Error: core::fmt::Display;

    /// Read from a socket.
    ///
    /// On completion, the number of bytes read is returned.
    fn read(&mut self, buf: &mut [u8]) -> impl Future<Output = Result<usize, Self::Error>>;

    /// Write to the socket.
    fn write(&mut self, buf: &[u8]) -> impl Future<Output = Result<(), Self::Error>>;
}
