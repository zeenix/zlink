#![cfg_attr(not(feature = "tokio"), no_std)]
#![deny(
    missing_debug_implementations,
    nonstandard_style,
    rust_2018_idioms,
    missing_docs
)]
#![warn(unreachable_pub)]
#![doc = include_str!("../README.md")]

#[cfg(not(feature = "tokio"))]
compile_error!(
    "Currently 'tokio' feature must be enabled. `embassy` feature will also be supported in the future."
);

#[cfg(feature = "tokio")]
pub use zlink_tokio::*;
