//! The low-level Socket read and write traits.

use core::future::Future;

/// The socket trait.
///
/// This is the trait that needs to be implemented for a type to be used as a socket/transport.
pub trait Socket: core::fmt::Debug {
    /// The read half of the socket.
    type ReadHalf: ReadHalf;
    /// The write half of the socket.
    type WriteHalf: WriteHalf;

    /// Split the socket into read and write halves.
    fn split(self) -> (Self::ReadHalf, Self::WriteHalf);
}

/// The read half of a socket.
///
/// Dropping the read half **must not** close the socket.
pub trait ReadHalf: core::fmt::Debug {
    /// Read from a socket.
    ///
    /// On completion, the number of bytes read is returned.
    ///
    /// Note for implementers: the future returned by this method must be cancellable.
    fn read(&mut self, buf: &mut [u8]) -> impl Future<Output = crate::Result<usize>>;
}

/// The write half of a socket.
///
/// Dropping the write half **must** close the socket.
pub trait WriteHalf: core::fmt::Debug {
    /// Write to the socket.
    ///
    /// Note for implementers: the future returned by this method must be cancellable.
    fn write(&mut self, buf: &[u8]) -> impl Future<Output = crate::Result<()>>;
}
