//! Type definitions for Varlink IDL.

mod type_ref;
pub use type_ref::TypeRef;

use core::fmt;
use serde::Serialize;

#[cfg(feature = "idl-parse")]
use serde::Deserialize;

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
    Object,
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
    Struct(List<'a, Field<'a>>),
}

impl<'a> fmt::Display for Type<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Bool => write!(f, "bool"),
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::String => write!(f, "string"),
            Type::Object => write!(f, "object"),
            Type::Optional(optional) => write!(f, "?{}", optional),
            Type::Array(array) => write!(f, "[]{}", array),
            Type::Map(map) => write!(f, "[string]{}", map),
            Type::Custom(name) => write!(f, "{}", name),
            Type::Enum(variants) => {
                write!(f, "(")?;
                let mut first = true;
                for variant in variants.iter() {
                    if !first {
                        write!(f, ", ")?;
                    }
                    first = false;
                    write!(f, "{}", variant)?;
                }
                write!(f, ")")
            }
            Type::Struct(fields) => {
                write!(f, "(")?;
                let mut first = true;
                for field in fields.iter() {
                    if !first {
                        write!(f, ", ")?;
                    }
                    first = false;
                    write!(f, "{}", field)?;
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

impl<'a> Serialize for Type<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

#[cfg(feature = "idl-parse")]
impl<'de, 'a> Deserialize<'de> for Type<'a>
where
    'de: 'a,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;
        super::parse::parse_type(s).map_err(serde::de::Error::custom)
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
        write!(buf, "{}", Type::Object).unwrap();
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
        write!(buf, "{}", Type::Optional(TypeRef::borrowed(&INT_TYPE))).unwrap();
        assert_eq!(buf, "?int");

        buf.clear();
        write!(buf, "{}", Type::Array(TypeRef::borrowed(&STRING_TYPE))).unwrap();
        assert_eq!(buf, "[]string");

        buf.clear();
        write!(buf, "{}", Type::Map(TypeRef::borrowed(&BOOL_TYPE))).unwrap();
        assert_eq!(buf, "[string]bool");

        // Test with owned variants
        #[cfg(feature = "std")]
        {
            assert_eq!(Type::Optional(TypeRef::new(Type::Int)).to_string(), "?int");
            assert_eq!(
                Type::Array(TypeRef::new(Type::String)).to_string(),
                "[]string"
            );

            // Test complex nested types
            let nested_type = Type::Array(TypeRef::new(Type::Optional(TypeRef::new(Type::String))));
            assert_eq!(nested_type.to_string(), "[]?string");

            // Test inline enum
            let enum_type = Type::Enum(List::from(vec!["one", "two", "three"]));
            assert_eq!(enum_type.to_string(), "(one, two, three)");

            // Test inline struct
            let struct_type = Type::Struct(List::from(vec![
                Field::new("first", &Type::Int),
                Field::new("second", &Type::String),
            ]));
            assert_eq!(struct_type.to_string(), "(first: int, second: string)");
        }
    }

    #[test]
    fn type_serialization() {
        let ty = Type::Array(TypeRef::borrowed(&Type::Int));
        #[cfg(feature = "std")]
        let json = serde_json::to_string(&ty).unwrap();
        #[cfg(feature = "embedded")]
        let json = {
            let mut buffer = [0u8; 16];
            let len = serde_json_core::to_slice(&ty, &mut buffer).unwrap();
            let vec = mayheap::Vec::<_, 16>::from_slice(&buffer[..len]).unwrap();
            mayheap::String::<16>::from_utf8(vec).unwrap()
        };
        assert_eq!(json, r#""[]int""#);
    }
}
