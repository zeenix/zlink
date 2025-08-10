//! Type implementations for external third-party crates.
//!
//! This module provides Type trait implementations for commonly used external crates,
//! gated behind their respective feature flags.

// ============================================================================
// UUID support
// ============================================================================

#[cfg(feature = "uuid")]
impl super::Type for uuid::Uuid {
    const TYPE: &'static crate::idl::Type<'static> = &crate::idl::Type::String;
}

// ============================================================================
// URL support
// ============================================================================

#[cfg(feature = "url")]
impl super::Type for url::Url {
    const TYPE: &'static crate::idl::Type<'static> = &crate::idl::Type::String;
}

// ============================================================================
// Bytes support
// ============================================================================

#[cfg(feature = "bytes")]
impl super::Type for bytes::Bytes {
    const TYPE: &'static crate::idl::Type<'static> = &crate::idl::Type::String;
}

#[cfg(feature = "bytes")]
impl super::Type for bytes::BytesMut {
    const TYPE: &'static crate::idl::Type<'static> = &crate::idl::Type::String;
}

// ============================================================================
// IndexMap support
// ============================================================================

#[cfg(feature = "indexmap")]
impl<V: super::Type> super::Type for indexmap::IndexMap<String, V> {
    const TYPE: &'static crate::idl::Type<'static> =
        &crate::idl::Type::Map(crate::idl::TypeRef::new(V::TYPE));
}

#[cfg(feature = "indexmap")]
impl<V: super::Type> super::Type for indexmap::IndexMap<&str, V> {
    const TYPE: &'static crate::idl::Type<'static> =
        &crate::idl::Type::Map(crate::idl::TypeRef::new(V::TYPE));
}

#[cfg(feature = "indexmap")]
impl<T: super::Type> super::Type for indexmap::IndexSet<T> {
    const TYPE: &'static crate::idl::Type<'static> =
        &crate::idl::Type::Array(crate::idl::TypeRef::new(T::TYPE));
}

// ============================================================================
// Chrono support
// ============================================================================

#[cfg(feature = "chrono")]
impl super::Type for chrono::NaiveDate {
    const TYPE: &'static crate::idl::Type<'static> = &crate::idl::Type::String;
}

#[cfg(feature = "chrono")]
impl super::Type for chrono::NaiveTime {
    const TYPE: &'static crate::idl::Type<'static> = &crate::idl::Type::String;
}

#[cfg(feature = "chrono")]
impl super::Type for chrono::NaiveDateTime {
    const TYPE: &'static crate::idl::Type<'static> = &crate::idl::Type::String;
}

#[cfg(feature = "chrono")]
impl<Tz: chrono::TimeZone> super::Type for chrono::DateTime<Tz> {
    const TYPE: &'static crate::idl::Type<'static> = &crate::idl::Type::String;
}

#[cfg(feature = "chrono")]
impl super::Type for chrono::Duration {
    const TYPE: &'static crate::idl::Type<'static> = &crate::idl::Type::Int;
}

// ============================================================================
// Time crate support
// ============================================================================

#[cfg(feature = "time")]
impl super::Type for time::Date {
    const TYPE: &'static crate::idl::Type<'static> = &crate::idl::Type::String;
}

#[cfg(feature = "time")]
impl super::Type for time::Time {
    const TYPE: &'static crate::idl::Type<'static> = &crate::idl::Type::String;
}

#[cfg(feature = "time")]
impl super::Type for time::PrimitiveDateTime {
    const TYPE: &'static crate::idl::Type<'static> = &crate::idl::Type::String;
}

#[cfg(feature = "time")]
impl super::Type for time::OffsetDateTime {
    const TYPE: &'static crate::idl::Type<'static> = &crate::idl::Type::String;
}

#[cfg(feature = "time")]
impl super::Type for time::Duration {
    const TYPE: &'static crate::idl::Type<'static> = &crate::idl::Type::Float;
}
