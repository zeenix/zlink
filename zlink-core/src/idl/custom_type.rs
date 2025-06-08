//! Custom type, field and parameter definitions for Varlink IDL.

use core::fmt;

use serde::{Deserialize, Serialize};

use super::{List, Type};

/// A field in a custom type or method parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field<'a> {
    /// The name of the field.
    pub name: &'a str,
    /// The type of the field.
    pub ty: Type<'a>,
}

/// Type alias for method parameters, which have the same structure as fields.
pub type Parameter<'a> = Field<'a>;

/// A custom type definition in Varlink IDL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomType<'a> {
    /// The name of the custom type.
    pub name: &'a str,
    /// The fields of the custom type.
    pub fields: List<'a, Field<'a>>,
}

impl<'a> CustomType<'a> {
    /// Creates a new custom type with the given name and fields.
    pub fn new(name: &'a str, fields: List<'a, Field<'a>>) -> Self {
        Self { name, fields }
    }
}

impl<'a> fmt::Display for CustomType<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "type {} (", self.name)?;
        let mut first = true;
        for field in self.fields.iter() {
            if !first {
                write!(f, ", ")?;
            }
            first = false;
            write!(f, "{}", field)?;
        }
        write!(f, ")")
    }
}

impl<'a> Serialize for CustomType<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de, 'a> Deserialize<'de> for CustomType<'a>
where
    'de: 'a,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;
        super::parse::parse_custom_type(s).map_err(serde::de::Error::custom)
    }
}

impl<'a> Field<'a> {
    /// Creates a new field with the given name and type.
    pub fn new(name: &'a str, ty: Type<'a>) -> Self {
        Self { name, ty }
    }
}

impl<'a> fmt::Display for Field<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.ty)
    }
}

impl<'a> Serialize for Field<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de, 'a> Deserialize<'de> for Field<'a>
where
    'de: 'a,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;
        super::parse::parse_field(s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_creation() {
        let field = Field::new("age", Type::Int);
        assert_eq!(field.name, "age");
        assert_eq!(field.ty, Type::Int);
    }

    #[test]
    fn test_field_serialization() {
        let field = Field::new("count", Type::Int);
        let json = serde_json::to_string(&field).unwrap();
        assert_eq!(json, r#""count: int""#);
    }

    #[test]
    fn test_parameter_alias() {
        let param: Parameter<'_> = Field::new("input", Type::String);
        assert_eq!(param.name, "input");
        assert_eq!(param.ty, Type::String);
    }

    #[test]
    fn test_custom_type_creation() {
        let fields = vec![Field::new("x", Type::Float), Field::new("y", Type::Float)];
        let custom_type = CustomType::new("Point", List::from(fields));
        assert_eq!(custom_type.name, "Point");
        assert_eq!(custom_type.fields.len(), 2);
    }

    #[test]
    fn test_custom_type_serialization() {
        let fields = vec![Field::new("x", Type::Float), Field::new("y", Type::Float)];
        let custom_type = CustomType::new("Point", List::from(fields));
        let json = serde_json::to_string(&custom_type).unwrap();
        assert_eq!(json, r#""type Point (x: float, y: float)""#);
    }
}
