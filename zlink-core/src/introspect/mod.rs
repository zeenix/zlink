//! Type introspection support for Varlink.
//!
//! This module provides traits and types for runtime type introspection in Varlink,
//! enabling the generation of type metadata and IDL information.

#![deny(missing_docs)]

mod r#type;
pub use r#type::Type;

// Re-export the Type derive macro so it's available alongside the trait
pub use zlink_macros::Type;

pub mod custom;
