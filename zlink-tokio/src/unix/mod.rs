//! Provides transport over Unix Domain Sockets.

mod stream;
pub use stream::{connect, Connection, Stream};
