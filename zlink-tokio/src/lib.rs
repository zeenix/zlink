#![deny(
    missing_debug_implementations,
    nonstandard_style,
    rust_2018_idioms,
    missing_docs
)]
#![warn(unreachable_pub)]
#![doc = include_str!("../../README.md")]

pub use zlink::*;
pub mod notified;
pub mod unix;
