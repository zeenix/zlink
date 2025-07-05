//! Types for the `org.varlink.service` interface.
//!
//! This module provides types for methods and errors to be used for both client and server
//! implementations of the standard Varlink service interface.

mod info;
pub use info::Info;
mod error;
pub use error::{Error, Result};

#[cfg(feature = "idl-parse")]
mod proxy;
#[cfg(feature = "idl-parse")]
pub use proxy::Proxy;

mod interface_description;
pub use interface_description::InterfaceDescription;
