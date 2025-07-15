#![cfg_attr(not(feature = "std"), no_std)]
#![deny(
    missing_debug_implementations,
    nonstandard_style,
    rust_2018_idioms,
    missing_docs
)]
#![warn(unreachable_pub, clippy::std_instead_of_core)]
#![doc = include_str!("../README.md")]

#[cfg(all(not(feature = "std"), not(feature = "embedded")))]
compile_error!("Either 'std' or 'embedded' feature must be enabled.");

#[macro_use]
mod log;

pub mod connection;
pub use connection::Connection;
mod error;
pub use error::{Error, Result};
mod server;
pub use server::{
    listener::Listener,
    service::{self, Service},
    Server,
};
mod call;
pub use call::Call;
pub mod reply;
pub use reply::Reply;
#[cfg(feature = "idl")]
pub mod idl;
#[cfg(feature = "introspection")]
pub mod introspect;
#[cfg(feature = "introspection")]
pub mod varlink_service;

#[cfg(test)]
mod test_utils;
