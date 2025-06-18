use super::{Type, TypeRef};

/// Type introspection.
pub trait TypeInfo {
    /// The type information.
    const TYPE_INFO: &'static Type<'static>;
}

/// Macro to implement TypeInfo for multiple types with the same Type variant.
macro_rules! impl_type_info {
    ($($ty:ty),* => $variant:expr) => {
        $(
            impl TypeInfo for $ty {
                const TYPE_INFO: &'static Type<'static> = &$variant;
            }
        )*
    };
}

// Implementations for primitive types.
impl_type_info!(bool => Type::Bool);
impl_type_info!(i8, i16, i32, i64, u8, u16, u32, u64 => Type::Int);
impl_type_info!(f32, f64 => Type::Float);
impl_type_info!(&str => Type::String);

#[cfg(feature = "std")]
impl_type_info!(String => Type::String);

// Optional types.
impl<T: TypeInfo> TypeInfo for Option<T> {
    const TYPE_INFO: &'static Type<'static> = &Type::Optional(TypeRef::new(T::TYPE_INFO));
}

// Array types.
#[cfg(feature = "std")]
impl<T: TypeInfo> TypeInfo for Vec<T> {
    const TYPE_INFO: &'static Type<'static> = &Type::Array(TypeRef::new(T::TYPE_INFO));
}

impl<T: TypeInfo> TypeInfo for &[T] {
    const TYPE_INFO: &'static Type<'static> = &Type::Array(TypeRef::new(T::TYPE_INFO));
}

// For raw objects.
#[cfg(feature = "std")]
impl TypeInfo for serde_json::Value {
    const TYPE_INFO: &'static Type<'static> = &Type::Object;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitive_type_info() {
        assert_eq!(*bool::TYPE_INFO, Type::Bool);
        assert_eq!(*i32::TYPE_INFO, Type::Int);
        assert_eq!(*f64::TYPE_INFO, Type::Float);
        assert_eq!(*<&str>::TYPE_INFO, Type::String);
        #[cfg(feature = "std")]
        {
            assert_eq!(*String::TYPE_INFO, Type::String);
        }
    }

    #[test]
    fn optional_type_info() {
        match <Option<i32>>::TYPE_INFO {
            Type::Optional(optional) => assert_eq!(*optional, Type::Int),
            _ => panic!("Expected optional type"),
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn array_type_info() {
        match <Vec<String>>::TYPE_INFO {
            Type::Array(array) => assert_eq!(*array, Type::String),
            _ => panic!("Expected array type"),
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn complex_type_info() {
        // Test Option<Vec<bool>>
        match <Option<Vec<bool>>>::TYPE_INFO {
            Type::Optional(optional) => match &**optional {
                Type::Array(array) => assert_eq!(*array, Type::Bool),
                _ => panic!("Expected array inside optional"),
            },
            _ => panic!("Expected optional type"),
        }
    }
}
