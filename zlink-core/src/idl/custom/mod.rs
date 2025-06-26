//! Custom type definitions for Varlink IDL.
//!
//! This module contains definitions for custom types in Varlink IDL, including
//! object types (struct-like with named fields) and enum types (with named variants).

mod object;
pub use object::Object;

mod r#enum;
pub use r#enum::Enum;

mod r#type;
pub use r#type::Type;
