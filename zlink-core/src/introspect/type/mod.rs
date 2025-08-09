//! Type introspection support.
//!
//! This module provides implementations of the [`Type`] trait for standard Rust types,
//! mapping them to their corresponding IDL representations.
//!
//! # Organization
//!
//! The implementations are organized into logical categories:
//! - `primitives`: Basic Rust types (bool, integers, floats, strings)
//! - `collections`: Container types (Vec, HashMap, HashSet, etc.)
//! - `wrappers`: Transparent wrapper types (Box, Arc, Option, Cell, etc.)
//! - `special`: Special standard library types (paths, network addresses, time, etc.)
//! - `external`: Third-party crate integrations (uuid, chrono, etc.)

use super::super::idl;

/// Type introspection.
///
/// This trait provides type metadata for Rust types, mapping them to their corresponding
/// IDL representation. Implementing this trait allows a type to participate in Varlink's
/// type introspection system.
///
/// # Usage
///
/// For custom types, use the `Type` derive macro:
///
/// ```ignore
/// use zlink::introspect::Type;
///
/// #[derive(Type)]
/// struct MyRequest {
///     id: String,
///     count: i32,
/// }
/// ```
///
/// The derive macro automatically generates the appropriate `Type` implementation based on
/// your struct's fields.
pub trait Type {
    /// The type information.
    const TYPE: &'static idl::Type<'static>;
}

// Macro utilities.
#[macro_use]
mod macros;

// Implementation modules.
mod collections;
mod external;
mod primitives;
mod special;
mod wrappers;

#[cfg(test)]
mod tests;
