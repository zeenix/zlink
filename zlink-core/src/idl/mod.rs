//! Interface Definition Language (IDL) support for Varlink.
//!
//! This module provides types and parsers for working with Varlink IDL definitions.

#![deny(missing_docs)]

mod list;
pub use list::List;

mod r#type;
pub use r#type::{Type, TypeRef};

mod custom_type;
pub use custom_type::CustomType;

mod field;
pub use field::{Field, Parameter};

mod method;
pub use method::Method;

mod error;
pub use error::Error;

mod member;
pub use member::Member;

mod interface;
pub use interface::Interface;

mod type_info;
pub use type_info::TypeInfo;

// Re-export the TypeInfo derive macro so it's available alongside the trait
pub use zlink_macros::TypeInfo;

mod reply_error;
pub use reply_error::ReplyError;

#[cfg(feature = "idl-parse")]
mod parse;
