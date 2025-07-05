//! Type definitions for Varlink IDL.

mod type_ref;
pub use type_ref::TypeRef;

use core::fmt;

use super::{Field, List};

/// Represents a type in Varlink IDL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type<'a> {
    /// Boolean type.
    Bool,
    /// 64-bit signed integer.
    Int,
    /// 64-bit floating point.
    Float,
    /// UTF-8 string.
    String,
    /// Foreign untyped object.
    ForeignObject,
    /// Optional/nullable type.
    Optional(TypeRef<'a>),
    /// Array type.
    Array(TypeRef<'a>),
    /// Map type with string keys.
    Map(TypeRef<'a>),
    /// Custom named type reference.
    Custom(&'a str),
    /// Inline enum type.
    Enum(List<'a, &'a str>),
    /// Inline struct type.
    Object(List<'a, Field<'a>>),
}

impl<'a> Type<'a> {
    /// The child type if this type is optional.
    pub const fn as_optional(&self) -> Option<&TypeRef<'a>> {
        match self {
            Type::Optional(optional) => Some(optional),
            _ => None,
        }
    }

    /// The array element type if this type is an array.
    pub const fn as_array(&self) -> Option<&TypeRef<'a>> {
        match self {
            Type::Array(array) => Some(array),
            _ => None,
        }
    }

    /// The map value type if this type is a map.
    pub const fn as_map(&self) -> Option<&TypeRef<'a>> {
        match self {
            Type::Map(map) => Some(map),
            _ => None,
        }
    }

    /// The custom type name if this type is a custom type.
    pub const fn as_custom(&self) -> Option<&'a str> {
        match self {
            Type::Custom(custom) => Some(custom),
            _ => None,
        }
    }

    /// The enum variants if this type is an enum.
    pub const fn as_enum(&self) -> Option<&List<'a, &'a str>> {
        match self {
            Type::Enum(variants) => Some(variants),
            _ => None,
        }
    }

    /// The object fields if this type is an object.
    pub const fn as_object(&self) -> Option<&List<'a, Field<'a>>> {
        match self {
            Type::Object(fields) => Some(fields),
            _ => None,
        }
    }
}

impl<'a> fmt::Display for Type<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Bool => write!(f, "bool"),
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::String => write!(f, "string"),
            Type::ForeignObject => write!(f, "object"),
            Type::Optional(optional) => write!(f, "?{optional}"),
            Type::Array(array) => write!(f, "[]{array}"),
            Type::Map(map) => write!(f, "[string]{map}"),
            Type::Custom(name) => write!(f, "{name}"),
            Type::Enum(variants) => {
                write!(f, "(")?;
                let mut first = true;
                for variant in variants.iter() {
                    if !first {
                        write!(f, ", ")?;
                    }
                    first = false;
                    write!(f, "{variant}")?;
                }
                write!(f, ")")
            }
            Type::Object(fields) => {
                write!(f, "(")?;
                let mut first = true;
                for field in fields.iter() {
                    if !first {
                        write!(f, ", ")?;
                    }
                    first = false;
                    write!(f, "{field}")?;
                }
                write!(f, ")")
            }
        }
    }
}

impl<'a> PartialEq<TypeRef<'a>> for Type<'a> {
    fn eq(&self, other: &TypeRef<'a>) -> bool {
        self == other.inner()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_names() {
        use core::fmt::Write;
        let mut buf = mayheap::String::<32>::new();

        buf.clear();
        write!(buf, "{}", Type::Bool).unwrap();
        assert_eq!(buf, "bool");

        buf.clear();
        write!(buf, "{}", Type::Int).unwrap();
        assert_eq!(buf, "int");

        buf.clear();
        write!(buf, "{}", Type::Float).unwrap();
        assert_eq!(buf, "float");

        buf.clear();
        write!(buf, "{}", Type::String).unwrap();
        assert_eq!(buf, "string");

        buf.clear();
        write!(buf, "{}", Type::ForeignObject).unwrap();
        assert_eq!(buf, "object");
    }

    #[test]
    fn complex_type_names() {
        // Test with const-friendly borrowed variants
        const INT_TYPE: Type<'static> = Type::Int;
        const STRING_TYPE: Type<'static> = Type::String;
        const BOOL_TYPE: Type<'static> = Type::Bool;

        use core::fmt::Write;
        let mut buf = mayheap::String::<64>::new();

        buf.clear();
        write!(buf, "{}", Type::Optional(TypeRef::new(&INT_TYPE))).unwrap();
        assert_eq!(buf, "?int");

        buf.clear();
        write!(buf, "{}", Type::Array(TypeRef::new(&STRING_TYPE))).unwrap();
        assert_eq!(buf, "[]string");

        buf.clear();
        write!(buf, "{}", Type::Map(TypeRef::new(&BOOL_TYPE))).unwrap();
        assert_eq!(buf, "[string]bool");

        // Test with owned variants
        #[cfg(feature = "std")]
        {
            assert_eq!(
                Type::Optional(TypeRef::new_owned(Type::Int)).to_string(),
                "?int"
            );
            assert_eq!(
                Type::Array(TypeRef::new_owned(Type::String)).to_string(),
                "[]string"
            );

            // Test complex nested types
            let nested_type = Type::Array(TypeRef::new_owned(Type::Optional(TypeRef::new_owned(
                Type::String,
            ))));
            assert_eq!(nested_type.to_string(), "[]?string");

            // Test inline enum
            let enum_type = Type::Enum(List::from(vec!["one", "two", "three"]));
            assert_eq!(enum_type.to_string(), "(one, two, three)");

            // Test inline struct
            let struct_type = Type::Object(List::from(vec![
                Field::new("first", &Type::Int, &[]),
                Field::new("second", &Type::String, &[]),
            ]));
            assert_eq!(struct_type.to_string(), "(first: int, second: string)");
        }
    }
}
