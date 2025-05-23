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

/// Documentation-only socket implementations for doc tests.
///
/// These types exist only to make doc tests compile and should never be used in real code.
#[doc(hidden)]
pub mod impl_for_doc {

    /// A mock socket for documentation examples.
    #[derive(Debug)]
    pub struct Socket;

    impl super::Socket for Socket {
        type ReadHalf = ReadHalf;
        type WriteHalf = WriteHalf;

        fn split(self) -> (Self::ReadHalf, Self::WriteHalf) {
            (ReadHalf, WriteHalf)
        }
    }

    /// A mock read half for documentation examples.
    #[derive(Debug)]
    pub struct ReadHalf;

    impl super::ReadHalf for ReadHalf {
        async fn read(&mut self, _buf: &mut [u8]) -> crate::Result<usize> {
            unreachable!("This is only for doc tests")
        }
    }

    /// A mock write half for documentation examples.
    #[derive(Debug)]
    pub struct WriteHalf;

    impl super::WriteHalf for WriteHalf {
        async fn write(&mut self, _buf: &[u8]) -> crate::Result<()> {
            unreachable!("This is only for doc tests")
        }
    }
}
