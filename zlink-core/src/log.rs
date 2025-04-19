//! Logging macros that abstract `tracing` and `defmt` one.
//!
//! Since these macros are internal API, we only have ones that we need.

#[cfg(feature = "tracing")]
macro_rules! warn {
    ($($arg:tt)*) => {
        tracing::warn!($($arg)*)
    }
}
// Note: Since user has to enable either `tracing` or `defmt` feature, we can assume that `defmt` is
// enabled when `tracing` is not.
#[cfg(not(feature = "tracing"))]
macro_rules! warn {
    ($($arg:tt)*) => {
        defmt::warn!($($arg)*)
    }
}

#[cfg(feature = "tracing")]
macro_rules! trace {
    ($($arg:tt)*) => {
        tracing::trace!($($arg)*)
    }
}
#[cfg(not(feature = "tracing"))]
macro_rules! trace {
    ($($arg:tt)*) => {
        defmt::trace!($($arg)*)
    }
}
