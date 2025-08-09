//! Macros for implementing the Type trait.

/// Implements the Type trait for multiple types mapping to the same IDL type.
///
/// This macro simplifies bulk implementation of the Type trait for groups of types
/// that share the same IDL representation.
///
/// # Examples
///
/// ```ignore
/// // Single type
/// impl_type!(bool => idl::Type::Bool);
///
/// // Multiple types with the same mapping
/// impl_type!(i8, i16, i32, i64 => idl::Type::Int);
///
/// // Feature-gated implementations
/// #[cfg(feature = "std")]
/// impl_type!(String => idl::Type::String);
/// ```
#[macro_export]
macro_rules! impl_type {
    // Single type
    ($ty:ty => $variant:expr) => {
        impl $crate::introspect::Type for $ty {
            const TYPE: &'static $crate::idl::Type<'static> = &$variant;
        }
    };
    // Multiple types
    ($($ty:ty),+ => $variant:expr) => {
        $(
            impl $crate::introspect::Type for $ty {
                const TYPE: &'static $crate::idl::Type<'static> = &$variant;
            }
        )+
    };
}

/// Implements the Type trait for a generic collection type.
///
/// This macro helps implement the Type trait for generic collection types
/// like Vec, HashSet, etc., that contain elements of a type implementing Type.
///
/// # Examples
///
/// ```ignore
/// // For array-like collections
/// impl_collection_type!(Vec<T> => Array);
/// impl_collection_type!(HashSet<T> => Array);
///
/// // For map-like collections (with string keys)
/// impl_map_type!(HashMap<String, V> => Map);
/// ```
#[macro_export]
macro_rules! impl_collection_type {
    ($ty:ident<$generic:ident> => Array) => {
        impl<$generic: $crate::introspect::Type> $crate::introspect::Type for $ty<$generic> {
            const TYPE: &'static $crate::idl::Type<'static> =
                &$crate::idl::Type::Array($crate::idl::TypeRef::new($generic::TYPE));
        }
    };
}

/// Implements the Type trait for map types with string keys.
///
/// This macro helps implement the Type trait for map types that have
/// string keys and values of a type implementing Type.
///
/// # Examples
///
/// ```ignore
/// impl_map_type!(HashMap<String, V>);
/// impl_map_type!(BTreeMap<&str, V>);
/// ```
#[macro_export]
macro_rules! impl_map_type {
    ($ty:ident<String, $value:ident>) => {
        impl<$value: $crate::introspect::Type> $crate::introspect::Type for $ty<String, $value> {
            const TYPE: &'static $crate::idl::Type<'static> =
                &$crate::idl::Type::Map($crate::idl::TypeRef::new($value::TYPE));
        }
    };
    ($ty:ident<&str, $value:ident>) => {
        impl<$value: $crate::introspect::Type> $crate::introspect::Type for $ty<&str, $value> {
            const TYPE: &'static $crate::idl::Type<'static> =
                &$crate::idl::Type::Map($crate::idl::TypeRef::new($value::TYPE));
        }
    };
}

/// Implements the Type trait for transparent wrapper types.
///
/// This macro helps implement the Type trait for types that transparently
/// wrap another type, like Box, Arc, Rc, Cell, etc.
///
/// # Examples
///
/// ```ignore
/// impl_transparent_wrapper!(Box<T>);
/// impl_transparent_wrapper!(Arc<T>);
/// impl_transparent_wrapper!(Cell<T>);
/// ```
#[macro_export]
macro_rules! impl_transparent_wrapper {
    ($wrapper:ident<$inner:ident>) => {
        impl<$inner: $crate::introspect::Type + ?Sized> $crate::introspect::Type
            for $wrapper<$inner>
        {
            const TYPE: &'static $crate::idl::Type<'static> = $inner::TYPE;
        }
    };
    // Variant for module-qualified types
    ($module:ident::$wrapper:ident<$inner:ident>) => {
        impl<$inner: $crate::introspect::Type + ?Sized> $crate::introspect::Type
            for $module::$wrapper<$inner>
        {
            const TYPE: &'static $crate::idl::Type<'static> = $inner::TYPE;
        }
    };
}
