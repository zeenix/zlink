use core::{
    pin::Pin,
    task::{Context, Poll},
};

use crate::Result;

/// The socket trait.
///
/// This is the trait that needs to be implemented for a type to be used as a socket/transport.
pub trait Socket {
    /// Read from a socket.
    ///
    /// On completion, the number of bytes read is returned.
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8])
        -> Poll<Result<usize>>;

    /// Write to the socket.
    ///
    /// On completion, the number of bytes written is returned.
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>>;
}
