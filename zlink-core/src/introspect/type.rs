use super::super::{idl, idl::TypeRef};

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
impl_type!(i8, i16, i32, i64, u8, u16, u32, u64 => idl::Type::Int);
impl_type!(f32, f64 => idl::Type::Float);
impl_type!(&str => idl::Type::String);

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

impl<T: Type> Type for &[T] {
    const TYPE: &'static idl::Type<'static> = &idl::Type::Array(TypeRef::new(T::TYPE));
}

// For raw objects.
#[cfg(feature = "std")]
impl Type for serde_json::Value {
    const TYPE: &'static idl::Type<'static> = &idl::Type::ForeignObject;
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
}
