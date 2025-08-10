//! Type implementations for collection types.

use crate::{idl, idl::TypeRef, introspect::Type};
#[cfg(feature = "std")]
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

// ============================================================================
// Array/Vector types
// ============================================================================

/// Standard Vec implementation.
#[cfg(feature = "std")]
impl<T: Type> Type for Vec<T> {
    const TYPE: &'static idl::Type<'static> = &idl::Type::Array(TypeRef::new(T::TYPE));
}

/// Heapless Vec implementation.
impl<T: Type, const N: usize> Type for mayheap::Vec<T, N> {
    const TYPE: &'static idl::Type<'static> = &idl::Type::Array(TypeRef::new(T::TYPE));
}

/// Slice implementation.
impl<T: Type> Type for &[T] {
    const TYPE: &'static idl::Type<'static> = &idl::Type::Array(TypeRef::new(T::TYPE));
}

// ============================================================================
// Map types - Varlink maps always have string keys
// ============================================================================

#[cfg(feature = "std")]
impl<V: Type> Type for HashMap<String, V> {
    const TYPE: &'static idl::Type<'static> = &idl::Type::Map(TypeRef::new(V::TYPE));
}

#[cfg(feature = "std")]
impl<V: Type> Type for HashMap<&str, V> {
    const TYPE: &'static idl::Type<'static> = &idl::Type::Map(TypeRef::new(V::TYPE));
}

#[cfg(feature = "std")]
impl<V: Type> Type for BTreeMap<String, V> {
    const TYPE: &'static idl::Type<'static> = &idl::Type::Map(TypeRef::new(V::TYPE));
}

#[cfg(feature = "std")]
impl<V: Type> Type for BTreeMap<&str, V> {
    const TYPE: &'static idl::Type<'static> = &idl::Type::Map(TypeRef::new(V::TYPE));
}

// ============================================================================
// Set types - represented as arrays in Varlink
// ============================================================================

#[cfg(feature = "std")]
impl<T: Type> Type for HashSet<T> {
    const TYPE: &'static idl::Type<'static> = &idl::Type::Array(TypeRef::new(T::TYPE));
}

#[cfg(feature = "std")]
impl<T: Type> Type for BTreeSet<T> {
    const TYPE: &'static idl::Type<'static> = &idl::Type::Array(TypeRef::new(T::TYPE));
}