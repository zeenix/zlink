#![deny(
    missing_debug_implementations,
    nonstandard_style,
    rust_2018_idioms,
    missing_docs
)]
#![warn(unreachable_pub)]
#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

pub use zlink_core::*;
pub mod notified;
pub mod unix;
