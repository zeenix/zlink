//! Type introspection support for Varlink.
//!
//! This module provides traits and types for runtime type introspection in Varlink,
//! enabling the generation of type metadata and IDL information.

#![deny(missing_docs)]

mod r#type;
pub use r#type::Type;

mod custom_type;
pub use custom_type::CustomType;

mod reply_error;
pub use reply_error::ReplyError;

// Re-export the the derive macro so it's available alongside the traits.
pub use zlink_macros::{CustomType, Type};
