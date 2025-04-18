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
pub trait ReadHalf: core::fmt::Debug {
    /// Read from a socket.
    ///
    /// On completion, the number of bytes read is returned.
    ///
    /// Notes for implementers:
    ///
    /// * The future returned by this method must be cancel safe.
    /// * While there is no explicit `Unpin` bound on the future returned by this method, it is
    ///   expected that it provides the same guarentees as `Unpin` would require. The reason `Unpin`
    ///   is not explicitly requied is that it would force boxing (and therefore allocation) on the
    ///   implemention that use `async fn`, which is undesirable for embedded use cases. See [this
    ///   issue](https://github.com/rust-lang/rust/issues/82187) for details.
    fn read(&mut self, buf: &mut [u8]) -> impl Future<Output = crate::Result<usize>>;
}

/// The write half of a socket.
pub trait WriteHalf: core::fmt::Debug {
    /// Write to the socket.
    ///
    /// The returned future has the same requirements as that of [`ReadHalf::read`].
    fn write(&mut self, buf: &[u8]) -> impl Future<Output = crate::Result<()>>;
}
