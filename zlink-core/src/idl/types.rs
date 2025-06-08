//! Type definitions for Varlink IDL.

use core::fmt;
use serde::{Deserialize, Serialize};

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;
#[cfg(feature = "std")]
use std::boxed::Box;

use super::{Field, List};

/// A type reference that can be either borrowed or owned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeRef<'a>(TypeRefInner<'a>);

#[derive(Debug, Clone, PartialEq, Eq)]
enum TypeRefInner<'a> {
    Borrowed(&'a Type<'a>),
    Owned(Box<Type<'a>>),
}

impl<'a> TypeRef<'a> {
    /// Creates a new type reference with an owned type.
    pub fn new(inner: Type<'a>) -> Self {
        Self(TypeRefInner::Owned(Box::new(inner)))
    }

    /// Creates a new type reference with a borrowed type reference.
    pub const fn borrowed(inner: &'a Type<'a>) -> Self {
        Self(TypeRefInner::Borrowed(inner))
    }

    /// Returns a reference to the inner type.
    pub fn inner(&self) -> &Type<'a> {
        match &self.0 {
            TypeRefInner::Borrowed(inner) => inner,
            TypeRefInner::Owned(inner) => inner,
        }
    }
}

impl<'a> fmt::Display for TypeRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner())
    }
}

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

impl<'a> Serialize for Type<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

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
    fn test_type_names() {
        assert_eq!(Type::Bool.to_string(), "bool");
        assert_eq!(Type::Int.to_string(), "int");
        assert_eq!(Type::Float.to_string(), "float");
        assert_eq!(Type::String.to_string(), "string");
        assert_eq!(Type::Object.to_string(), "object");
    }

    #[test]
    fn test_complex_type_names() {
        // Test with const-friendly borrowed variants
        const INT_TYPE: Type<'static> = Type::Int;
        const STRING_TYPE: Type<'static> = Type::String;
        const BOOL_TYPE: Type<'static> = Type::Bool;

        assert_eq!(
            Type::Optional(TypeRef::borrowed(&INT_TYPE)).to_string(),
            "?int"
        );
        assert_eq!(
            Type::Array(TypeRef::borrowed(&STRING_TYPE)).to_string(),
            "[]string"
        );
        assert_eq!(
            Type::Map(TypeRef::borrowed(&BOOL_TYPE)).to_string(),
            "[string]bool"
        );

        // Test with owned variants
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
            Field::new("first", Type::Int),
            Field::new("second", Type::String),
        ]));
        assert_eq!(struct_type.to_string(), "(first: int, second: string)");
    }

    #[test]
    fn test_type_serialization() {
        let ty = Type::Array(TypeRef::new(Type::Int));
        let json = serde_json::to_string(&ty).unwrap();
        assert_eq!(json, r#""[]int""#);
    }
}
