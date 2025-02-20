#![no_std]
#![deny(
    missing_debug_implementations,
    nonstandard_style,
    rust_2018_idioms,
    missing_docs
)]
#![warn(unreachable_pub)]
#![doc = include_str!("../../README.md")]

#[cfg(not(feature = "alloc"))]
compile_error!("Currently the `alloc` feature is required");

pub mod connection;
mod error;
pub use error::{Error, Result};
