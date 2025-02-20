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

/// Test add.
pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
