//! Varlink names and their validation.
//!
//! Varlink names fields, types and interfaces each by their own rule. This crate checks whether a
//! string is a valid name of each kind. It is standalone: it does not pull in the rest of zlink.
//!
//! # Examples
//!
//! ```
//! use zlink_names::{is_valid_interface_name, is_valid_type_name};
//!
//! assert!(is_valid_interface_name("org.example.Foo"));
//! assert!(!is_valid_type_name("fileNotFound"));
//! ```
#![cfg_attr(not(test), no_std)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/z-galaxy/zlink/3660d731d7de8f60c8d82e122b3ece15617185e4/data/logo.png"
)]
#![deny(
    missing_debug_implementations,
    nonstandard_style,
    rust_2018_idioms,
    missing_docs
)]
#![warn(unreachable_pub, clippy::std_instead_of_core)]
#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

mod field_name;
pub use field_name::is_valid_field_name;

mod type_name;
pub use type_name::is_valid_type_name;

mod interface_name;
pub use interface_name::is_valid_interface_name;

#[doc(hidden)]
pub mod parse;
