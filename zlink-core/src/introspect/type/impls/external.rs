//! Type implementations for external third-party crates.
//!
//! This module provides Type trait implementations for commonly used external crates,
//! gated behind their respective feature flags.

use crate::{idl, introspect::Type};

// ============================================================================
// UUID support
// ============================================================================

#[cfg(feature = "uuid")]
impl Type for uuid::Uuid {
    const TYPE: &'static idl::Type<'static> = &idl::Type::String;
}

// ============================================================================
// URL support
// ============================================================================

#[cfg(feature = "url")]
impl Type for url::Url {
    const TYPE: &'static idl::Type<'static> = &idl::Type::String;
}

// ============================================================================
// Bytes support
// ============================================================================

#[cfg(feature = "bytes")]
impl Type for bytes::Bytes {
    const TYPE: &'static idl::Type<'static> = &idl::Type::String;
}

#[cfg(feature = "bytes")]
impl Type for bytes::BytesMut {
    const TYPE: &'static idl::Type<'static> = &idl::Type::String;
}

// ============================================================================
// IndexMap support
// ============================================================================

#[cfg(feature = "indexmap")]
mod indexmap_impls {
    use super::*;
    use crate::idl::TypeRef;

    impl<V: Type> Type for indexmap::IndexMap<String, V> {
        const TYPE: &'static idl::Type<'static> = &idl::Type::Map(TypeRef::new(V::TYPE));
    }

    impl<V: Type> Type for indexmap::IndexMap<&str, V> {
        const TYPE: &'static idl::Type<'static> = &idl::Type::Map(TypeRef::new(V::TYPE));
    }

    impl<T: Type> Type for indexmap::IndexSet<T> {
        const TYPE: &'static idl::Type<'static> = &idl::Type::Array(TypeRef::new(T::TYPE));
    }
}

// ============================================================================
// Chrono support
// ============================================================================

#[cfg(feature = "chrono")]
mod chrono_impls {
    use super::*;

    impl Type for chrono::NaiveDate {
        const TYPE: &'static idl::Type<'static> = &idl::Type::String;
    }

    impl Type for chrono::NaiveTime {
        const TYPE: &'static idl::Type<'static> = &idl::Type::String;
    }

    impl Type for chrono::NaiveDateTime {
        const TYPE: &'static idl::Type<'static> = &idl::Type::String;
    }

    impl<Tz: chrono::TimeZone> Type for chrono::DateTime<Tz> {
        const TYPE: &'static idl::Type<'static> = &idl::Type::String;
    }

    impl Type for chrono::Duration {
        const TYPE: &'static idl::Type<'static> = &idl::Type::Int;
    }
}

// ============================================================================
// Time crate support
// ============================================================================

#[cfg(feature = "time")]
mod time_impls {
    use super::*;

    impl Type for time::Date {
        const TYPE: &'static idl::Type<'static> = &idl::Type::String;
    }

    impl Type for time::Time {
        const TYPE: &'static idl::Type<'static> = &idl::Type::String;
    }

    impl Type for time::PrimitiveDateTime {
        const TYPE: &'static idl::Type<'static> = &idl::Type::String;
    }

    impl Type for time::OffsetDateTime {
        const TYPE: &'static idl::Type<'static> = &idl::Type::String;
    }

    impl Type for time::Duration {
        const TYPE: &'static idl::Type<'static> = &idl::Type::Float;
    }
}