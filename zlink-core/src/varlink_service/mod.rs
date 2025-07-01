//! Types for the `org.varlink.service` interface.
//!
//! This module provides types for methods and errors to be used for both client and server
//! implementations of the standard Varlink service interface.

mod info;
pub use info::Info;
mod error;
pub use error::{Error, Result};

// TODO: Implement introspection traits using derive macros once #[zlink(crate = "...")]
// attribute support is working properly for types with lifetimes.
// See: https://github.com/zeenix/zlink/issues/...
//
// The derive macros should generate:
// #[cfg_attr(feature = "introspection", derive(Type))]
// for Info<'a> struct
//
// #[cfg_attr(feature = "introspection", derive(ReplyError))]
// for Error<'a> enum
