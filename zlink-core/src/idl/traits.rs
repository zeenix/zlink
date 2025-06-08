//! Traits for type information and error handling in Varlink IDL.

use super::{Error, Type, TypeRef};

/// Provides type information for types that can be used in Varlink interfaces.
pub trait TypeInfo {
    /// The static type information for this type.
    const TYPE_INFO: &'static Type<'static>;
}

// Implementations for primitive types.
impl TypeInfo for bool {
    const TYPE_INFO: &'static Type<'static> = &Type::Bool;
}

impl TypeInfo for i8 {
    const TYPE_INFO: &'static Type<'static> = &Type::Int;
}

impl TypeInfo for i16 {
    const TYPE_INFO: &'static Type<'static> = &Type::Int;
}

impl TypeInfo for i32 {
    const TYPE_INFO: &'static Type<'static> = &Type::Int;
}

impl TypeInfo for i64 {
    const TYPE_INFO: &'static Type<'static> = &Type::Int;
}

impl TypeInfo for u8 {
    const TYPE_INFO: &'static Type<'static> = &Type::Int;
}

impl TypeInfo for u16 {
    const TYPE_INFO: &'static Type<'static> = &Type::Int;
}

impl TypeInfo for u32 {
    const TYPE_INFO: &'static Type<'static> = &Type::Int;
}

impl TypeInfo for u64 {
    const TYPE_INFO: &'static Type<'static> = &Type::Int;
}

impl TypeInfo for f32 {
    const TYPE_INFO: &'static Type<'static> = &Type::Float;
}

impl TypeInfo for f64 {
    const TYPE_INFO: &'static Type<'static> = &Type::Float;
}

impl TypeInfo for &str {
    const TYPE_INFO: &'static Type<'static> = &Type::String;
}

impl TypeInfo for String {
    const TYPE_INFO: &'static Type<'static> = &Type::String;
}

// Optional types.
impl<T: TypeInfo> TypeInfo for Option<T> {
    const TYPE_INFO: &'static Type<'static> = &Type::Optional(TypeRef::borrowed(T::TYPE_INFO));
}

// Array types.
impl<T: TypeInfo> TypeInfo for Vec<T> {
    const TYPE_INFO: &'static Type<'static> = &Type::Array(TypeRef::borrowed(T::TYPE_INFO));
}

impl<T: TypeInfo> TypeInfo for &[T] {
    const TYPE_INFO: &'static Type<'static> = &Type::Array(TypeRef::borrowed(T::TYPE_INFO));
}

// For raw objects.
impl TypeInfo for serde_json::Value {
    const TYPE_INFO: &'static Type<'static> = &Type::Object;
}

/// Trait for types that can have reply errors in Varlink methods.
pub trait ReplyErrors {
    /// The static list of possible errors this type can return.
    const REPLY_ERRORS: &'static [&'static Error<'static>];
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::idl::List;

    #[test]
    fn test_primitive_type_info() {
        assert_eq!(*bool::TYPE_INFO, Type::Bool);
        assert_eq!(*i32::TYPE_INFO, Type::Int);
        assert_eq!(*f64::TYPE_INFO, Type::Float);
        assert_eq!(*<&str>::TYPE_INFO, Type::String);
        assert_eq!(*String::TYPE_INFO, Type::String);
    }

    #[test]
    fn test_optional_type_info() {
        match *<Option<i32>>::TYPE_INFO {
            Type::Optional(ref optional) => assert_eq!(*optional.inner(), Type::Int),
            _ => panic!("Expected optional type"),
        }
    }

    #[test]
    fn test_array_type_info() {
        match *<Vec<String>>::TYPE_INFO {
            Type::Array(ref array) => assert_eq!(*array.inner(), Type::String),
            _ => panic!("Expected array type"),
        }
    }

    #[test]
    fn test_complex_type_info() {
        // Test Option<Vec<bool>>
        match *<Option<Vec<bool>>>::TYPE_INFO {
            Type::Optional(ref optional) => match optional.inner() {
                Type::Array(ref array) => assert_eq!(*array.inner(), Type::Bool),
                _ => panic!("Expected array inside optional"),
            },
            _ => panic!("Expected optional type"),
        }
    }

    #[test]
    fn test_reply_errors() {
        // Test with a type that implements ReplyErrors
        struct MyType;

        const MY_ERROR: Error<'static> = Error {
            name: "MyError",
            fields: List::Borrowed(&[]),
        };

        impl ReplyErrors for MyType {
            const REPLY_ERRORS: &'static [&'static Error<'static>] = &[&MY_ERROR];
        }

        assert_eq!(MyType::REPLY_ERRORS.len(), 1);
        assert_eq!(MyType::REPLY_ERRORS[0].name, "MyError");
    }
}
