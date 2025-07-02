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
