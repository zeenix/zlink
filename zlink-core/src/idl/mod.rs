//! Interface Definition Language (IDL) support for Varlink.
//!
//! This module provides types and parsers for working with Varlink IDL definitions.

#![deny(missing_docs)]

mod list;
pub use list::List;

mod types;
pub use types::{Type, TypeRef};

mod custom_type;
pub use custom_type::{CustomType, Field, Parameter};

mod method;
pub use method::Method;

mod error;
pub use error::Error;

mod member;
pub use member::Member;

mod interface;
pub use interface::Interface;

mod traits;
pub use traits::{ReplyErrors, TypeInfo};

mod parse;
