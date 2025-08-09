use super::super::{idl, idl::TypeRef};
#[cfg(feature = "std")]
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// Type introspection.
pub trait Type {
    /// The type information.
    const TYPE: &'static idl::Type<'static>;
}

/// Macro to implement Type for multiple types with the same idl::Type variant.
macro_rules! impl_type {
    ($($ty:ty),* => $variant:expr) => {
        $(
            impl Type for $ty {
                const TYPE: &'static idl::Type<'static> = &$variant;
            }
        )*
    };
}

// Implementations for primitive types.
impl_type!(bool => idl::Type::Bool);
impl_type!(i8, i16, i32, i64, u8, u16, u32, u64, isize, usize => idl::Type::Int);
impl_type!(f32, f64 => idl::Type::Float);
impl_type!(&str => idl::Type::String);
impl_type!(str => idl::Type::String);
impl_type!(char => idl::Type::String);

#[cfg(feature = "std")]
impl_type!(String => idl::Type::String);

// Optional types.
impl<T: Type> Type for Option<T> {
    const TYPE: &'static idl::Type<'static> = &idl::Type::Optional(TypeRef::new(T::TYPE));
}

// Array types.
#[cfg(feature = "std")]
impl<T: Type> Type for Vec<T> {
    const TYPE: &'static idl::Type<'static> = &idl::Type::Array(TypeRef::new(T::TYPE));
}

impl<T: Type, const N: usize> Type for mayheap::Vec<T, N> {
    const TYPE: &'static idl::Type<'static> = &idl::Type::Array(TypeRef::new(T::TYPE));
}

impl<const N: usize> Type for mayheap::string::String<N> {
    const TYPE: &'static idl::Type<'static> = &idl::Type::String;
}

impl<T: Type> Type for &[T] {
    const TYPE: &'static idl::Type<'static> = &idl::Type::Array(TypeRef::new(T::TYPE));
}

#[cfg(feature = "std")]
impl<T: Type + ToOwned + ?Sized> Type for std::borrow::Cow<'_, T> {
    const TYPE: &'static idl::Type<'static> = T::TYPE;
}

// Map types (Varlink maps always have string keys).
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

// Set types (represented as arrays in Varlink).
#[cfg(feature = "std")]
impl<T: Type> Type for HashSet<T> {
    const TYPE: &'static idl::Type<'static> = &idl::Type::Array(TypeRef::new(T::TYPE));
}

#[cfg(feature = "std")]
impl<T: Type> Type for BTreeSet<T> {
    const TYPE: &'static idl::Type<'static> = &idl::Type::Array(TypeRef::new(T::TYPE));
}

// Smart pointer types (transparent wrappers).
#[cfg(feature = "std")]
impl<T: Type + ?Sized> Type for Box<T> {
    const TYPE: &'static idl::Type<'static> = T::TYPE;
}

#[cfg(feature = "std")]
impl<T: Type + ?Sized> Type for std::rc::Rc<T> {
    const TYPE: &'static idl::Type<'static> = T::TYPE;
}

#[cfg(feature = "std")]
impl<T: Type + ?Sized> Type for std::sync::Arc<T> {
    const TYPE: &'static idl::Type<'static> = T::TYPE;
}

// Cell types (transparent wrappers).
#[cfg(feature = "std")]
impl<T: Type + ?Sized> Type for std::cell::Cell<T> {
    const TYPE: &'static idl::Type<'static> = T::TYPE;
}

#[cfg(feature = "std")]
impl<T: Type + ?Sized> Type for std::cell::RefCell<T> {
    const TYPE: &'static idl::Type<'static> = T::TYPE;
}

// Unit type maps to null/empty object.
impl Type for () {
    const TYPE: &'static idl::Type<'static> = &idl::Type::Object(idl::List::Borrowed(&[]));
}

// For raw objects.
#[cfg(feature = "std")]
impl Type for serde_json::Value {
    const TYPE: &'static idl::Type<'static> = &idl::Type::ForeignObject;
}

// Core time types (available in no-std).
impl Type for core::time::Duration {
    const TYPE: &'static idl::Type<'static> = &idl::Type::Float;
}

// Standard library time types (require std).
#[cfg(feature = "std")]
impl Type for std::time::Instant {
    const TYPE: &'static idl::Type<'static> = &idl::Type::Float;
}

#[cfg(feature = "std")]
impl Type for std::time::SystemTime {
    const TYPE: &'static idl::Type<'static> = &idl::Type::Float;
}

// Path types.
#[cfg(feature = "std")]
impl Type for std::path::PathBuf {
    const TYPE: &'static idl::Type<'static> = &idl::Type::String;
}

#[cfg(feature = "std")]
impl Type for std::path::Path {
    const TYPE: &'static idl::Type<'static> = &idl::Type::String;
}

// OsString types.
#[cfg(feature = "std")]
impl Type for std::ffi::OsString {
    const TYPE: &'static idl::Type<'static> = &idl::Type::String;
}

#[cfg(feature = "std")]
impl Type for std::ffi::OsStr {
    const TYPE: &'static idl::Type<'static> = &idl::Type::String;
}

// Network types
impl Type for core::net::IpAddr {
    const TYPE: &'static idl::Type<'static> = &idl::Type::String;
}

impl Type for core::net::Ipv4Addr {
    const TYPE: &'static idl::Type<'static> = &idl::Type::String;
}

impl Type for core::net::Ipv6Addr {
    const TYPE: &'static idl::Type<'static> = &idl::Type::String;
}

impl Type for core::net::SocketAddr {
    const TYPE: &'static idl::Type<'static> = &idl::Type::String;
}

impl Type for core::net::SocketAddrV4 {
    const TYPE: &'static idl::Type<'static> = &idl::Type::String;
}

impl Type for core::net::SocketAddrV6 {
    const TYPE: &'static idl::Type<'static> = &idl::Type::String;
}

// External type implementations for common third-party crates.

// UUID support.
#[cfg(feature = "uuid")]
impl Type for uuid::Uuid {
    const TYPE: &'static idl::Type<'static> = &idl::Type::String;
}

// Chrono support.
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

// Time crate support.
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

// URL support.
#[cfg(feature = "url")]
impl Type for url::Url {
    const TYPE: &'static idl::Type<'static> = &idl::Type::String;
}

// Bytes support.
#[cfg(feature = "bytes")]
impl Type for bytes::Bytes {
    const TYPE: &'static idl::Type<'static> = &idl::Type::String;
}

#[cfg(feature = "bytes")]
impl Type for bytes::BytesMut {
    const TYPE: &'static idl::Type<'static> = &idl::Type::String;
}

// IndexMap support.
#[cfg(feature = "indexmap")]
impl<V: Type> Type for indexmap::IndexMap<String, V> {
    const TYPE: &'static idl::Type<'static> = &idl::Type::Map(TypeRef::new(V::TYPE));
}

#[cfg(feature = "indexmap")]
impl<V: Type> Type for indexmap::IndexMap<&str, V> {
    const TYPE: &'static idl::Type<'static> = &idl::Type::Map(TypeRef::new(V::TYPE));
}

#[cfg(feature = "indexmap")]
impl<T: Type> Type for indexmap::IndexSet<T> {
    const TYPE: &'static idl::Type<'static> = &idl::Type::Array(TypeRef::new(T::TYPE));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitive_type() {
        assert_eq!(*bool::TYPE, idl::Type::Bool);
        assert_eq!(*i32::TYPE, idl::Type::Int);
        assert_eq!(*f64::TYPE, idl::Type::Float);
        assert_eq!(*<&str>::TYPE, idl::Type::String);
        #[cfg(feature = "std")]
        {
            assert_eq!(*String::TYPE, idl::Type::String);
        }
    }

    #[test]
    fn optional_type() {
        match <Option<i32>>::TYPE {
            idl::Type::Optional(optional) => assert_eq!(*optional, idl::Type::Int),
            _ => panic!("Expected optional type"),
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn array_type() {
        match <Vec<String>>::TYPE {
            idl::Type::Array(array) => assert_eq!(*array, idl::Type::String),
            _ => panic!("Expected array type"),
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn complex_type() {
        // Test Option<Vec<bool>>
        match <Option<Vec<bool>>>::TYPE {
            idl::Type::Optional(optional) => match &**optional {
                idl::Type::Array(array) => assert_eq!(*array, idl::Type::Bool),
                _ => panic!("Expected array inside optional"),
            },
            _ => panic!("Expected optional type"),
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn map_types() {
        // Test HashMap<String, i32>
        match <HashMap<String, i32>>::TYPE {
            idl::Type::Map(value_type) => assert_eq!(*value_type, idl::Type::Int),
            _ => panic!("Expected map type"),
        }

        // Test BTreeMap<&str, bool>
        match <BTreeMap<&str, bool>>::TYPE {
            idl::Type::Map(value_type) => assert_eq!(*value_type, idl::Type::Bool),
            _ => panic!("Expected map type"),
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn set_types() {
        // Test HashSet<String>
        match <HashSet<String>>::TYPE {
            idl::Type::Array(element_type) => assert_eq!(*element_type, idl::Type::String),
            _ => panic!("Expected array type"),
        }

        // Test BTreeSet<i32>
        match <BTreeSet<i32>>::TYPE {
            idl::Type::Array(element_type) => assert_eq!(*element_type, idl::Type::Int),
            _ => panic!("Expected array type"),
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn smart_pointer_types() {
        // Test Box<bool>
        assert_eq!(*<Box<bool>>::TYPE, idl::Type::Bool);

        // Test Arc<String>
        assert_eq!(*<std::sync::Arc<String>>::TYPE, idl::Type::String);

        // Test Rc<i32>
        assert_eq!(*<std::rc::Rc<i32>>::TYPE, idl::Type::Int);
    }

    #[cfg(feature = "std")]
    #[test]
    fn cell_types() {
        // Test Cell<f64>
        assert_eq!(*<std::cell::Cell<f64>>::TYPE, idl::Type::Float);

        // Test RefCell<bool>
        assert_eq!(*<std::cell::RefCell<bool>>::TYPE, idl::Type::Bool);
    }

    #[test]
    fn additional_numeric_types() {
        assert_eq!(*isize::TYPE, idl::Type::Int);
        assert_eq!(*usize::TYPE, idl::Type::Int);
    }

    #[test]
    fn char_type() {
        assert_eq!(*char::TYPE, idl::Type::String);
    }

    #[test]
    fn unit_type() {
        match <()>::TYPE {
            idl::Type::Object(fields) => assert_eq!(fields.iter().count(), 0),
            _ => panic!("Expected empty object type"),
        }
    }

    #[test]
    fn core_time_types() {
        // Test core::time::Duration (available in no-std)
        assert_eq!(*<core::time::Duration>::TYPE, idl::Type::Float);
    }

    #[cfg(feature = "std")]
    #[test]
    fn std_time_types() {
        // Test std-only time types
        assert_eq!(*<std::time::Instant>::TYPE, idl::Type::Float);
        assert_eq!(*<std::time::SystemTime>::TYPE, idl::Type::Float);
    }

    #[cfg(feature = "std")]
    #[test]
    fn path_types() {
        // Test path types
        assert_eq!(*<std::path::PathBuf>::TYPE, idl::Type::String);
        assert_eq!(*<std::path::Path>::TYPE, idl::Type::String);
    }

    #[cfg(feature = "std")]
    #[test]
    fn osstring_types() {
        // Test OsString types
        assert_eq!(*<std::ffi::OsString>::TYPE, idl::Type::String);
        assert_eq!(*<std::ffi::OsStr>::TYPE, idl::Type::String);
    }

    #[test]
    fn net_types() {
        // Test network address types (available in core)
        assert_eq!(*<core::net::IpAddr>::TYPE, idl::Type::String);
        assert_eq!(*<core::net::Ipv4Addr>::TYPE, idl::Type::String);
        assert_eq!(*<core::net::Ipv6Addr>::TYPE, idl::Type::String);
        assert_eq!(*<core::net::SocketAddr>::TYPE, idl::Type::String);
        assert_eq!(*<core::net::SocketAddrV4>::TYPE, idl::Type::String);
        assert_eq!(*<core::net::SocketAddrV6>::TYPE, idl::Type::String);
    }

    #[cfg(feature = "uuid")]
    #[test]
    fn uuid_type() {
        assert_eq!(*<uuid::Uuid>::TYPE, idl::Type::String);
    }

    #[cfg(feature = "url")]
    #[test]
    fn url_type() {
        assert_eq!(*<url::Url>::TYPE, idl::Type::String);
    }

    #[cfg(feature = "bytes")]
    #[test]
    fn bytes_types() {
        assert_eq!(*<bytes::Bytes>::TYPE, idl::Type::String);
        assert_eq!(*<bytes::BytesMut>::TYPE, idl::Type::String);
    }

    #[cfg(feature = "indexmap")]
    #[test]
    fn indexmap_types() {
        // Test IndexMap
        match <indexmap::IndexMap<String, i32>>::TYPE {
            idl::Type::Map(value_type) => assert_eq!(*value_type, idl::Type::Int),
            _ => panic!("Expected map type"),
        }

        match <indexmap::IndexMap<&str, bool>>::TYPE {
            idl::Type::Map(value_type) => assert_eq!(*value_type, idl::Type::Bool),
            _ => panic!("Expected map type"),
        }

        // Test IndexSet
        match <indexmap::IndexSet<String>>::TYPE {
            idl::Type::Array(element_type) => assert_eq!(*element_type, idl::Type::String),
            _ => panic!("Expected array type"),
        }
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn chrono_types() {
        assert_eq!(*<chrono::NaiveDate>::TYPE, idl::Type::String);
        assert_eq!(*<chrono::NaiveTime>::TYPE, idl::Type::String);
        assert_eq!(*<chrono::NaiveDateTime>::TYPE, idl::Type::String);
        assert_eq!(*<chrono::DateTime<chrono::Utc>>::TYPE, idl::Type::String);
        assert_eq!(*<chrono::Duration>::TYPE, idl::Type::Int);
    }

    #[cfg(feature = "time")]
    #[test]
    fn time_crate_types() {
        assert_eq!(*<time::Date>::TYPE, idl::Type::String);
        assert_eq!(*<time::Time>::TYPE, idl::Type::String);
        assert_eq!(*<time::PrimitiveDateTime>::TYPE, idl::Type::String);
        assert_eq!(*<time::OffsetDateTime>::TYPE, idl::Type::String);
        assert_eq!(*<time::Duration>::TYPE, idl::Type::Float);
    }
}
