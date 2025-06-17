//! Custom type, field and parameter definitions for Varlink IDL.

use core::fmt;

use serde::Serialize;

#[cfg(feature = "idl-parse")]
use serde::Deserialize;

use super::{List, Type, TypeRef};

/// A field in a custom type or method parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field<'a> {
    /// The name of the field.
    name: &'a str,
    /// The type of the field.
    ty: TypeRef<'a>,
}

/// Type alias for method parameters, which have the same structure as fields.
pub type Parameter<'a> = Field<'a>;

/// A custom type definition in Varlink IDL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomType<'a> {
    /// The name of the custom type.
    name: &'a str,
    /// The fields of the custom type.
    fields: List<'a, Field<'a>>,
}

impl<'a> CustomType<'a> {
    /// Creates a new custom type with the given name and borrowed fields.
    pub const fn new(name: &'a str, fields: &'a [&'a Field<'a>]) -> Self {
        Self {
            name,
            fields: List::Borrowed(fields),
        }
    }

    /// Creates a new custom type with the given name and owned fields.
    #[cfg(feature = "std")]
    pub fn new_owned(name: &'a str, fields: Vec<Field<'a>>) -> Self {
        Self {
            name,
            fields: List::Owned(fields),
        }
    }

    /// Returns the name of the custom type.
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// Returns an iterator over the fields of the custom type.
    pub fn fields(&self) -> impl Iterator<Item = &Field<'a>> {
        self.fields.iter()
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

#[cfg(feature = "idl-parse")]
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
    pub const fn new(name: &'a str, ty: &'a Type<'a>) -> Self {
        Self {
            name,
            ty: TypeRef::borrowed(ty),
        }
    }

    /// Same as `new` but takes `ty` by value.
    pub fn new_owned(name: &'a str, ty: Type<'a>) -> Self {
        Self {
            name,
            ty: TypeRef::new(ty),
        }
    }

    /// Returns the name of the field.
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// Returns the type of the field.
    pub fn ty(&self) -> &Type<'a> {
        self.ty.inner()
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

#[cfg(feature = "idl-parse")]
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
    use crate::idl::{Type, TypeInfo};

    #[test]
    fn field_creation() {
        let field = Field::new("age", <i32>::TYPE_INFO);
        assert_eq!(field.name(), "age");
        assert_eq!(field.ty(), &Type::Int);
    }

    #[test]
    fn field_serialization() {
        let field = Field::new("count", <i32>::TYPE_INFO);
        #[cfg(feature = "std")]
        let json = serde_json::to_string(&field).unwrap();
        #[cfg(feature = "embedded")]
        let json = {
            let mut buffer = [0u8; 16];
            let len = serde_json_core::to_slice(&field, &mut buffer).unwrap();
            let vec = mayheap::Vec::<_, 16>::from_slice(&buffer[..len]).unwrap();
            mayheap::String::<16>::from_utf8(vec).unwrap()
        };
        assert_eq!(json, r#""count: int""#);
    }

    #[test]
    fn parameter_alias() {
        let param: Parameter<'_> = Field::new("input", <&str>::TYPE_INFO);
        assert_eq!(param.name(), "input");
        assert_eq!(param.ty(), &Type::String);
    }

    #[test]
    fn custom_type_creation() {
        let field_x = Field::new("x", <f64>::TYPE_INFO);
        let field_y = Field::new("y", <f64>::TYPE_INFO);
        let fields = [&field_x, &field_y];

        let custom_type = CustomType::new("Point", &fields);
        assert_eq!(custom_type.name(), "Point");
        assert_eq!(custom_type.fields().count(), 2);

        // Check the fields individually - order and values.
        let fields_vec: mayheap::Vec<_, 8> = custom_type.fields().collect();
        assert_eq!(fields_vec[0].name(), "x");
        assert_eq!(fields_vec[0].ty(), &Type::Float);
        assert_eq!(fields_vec[1].name(), "y");
        assert_eq!(fields_vec[1].ty(), &Type::Float);
    }

    #[test]
    fn custom_type_serialization() {
        let field_x = Field::new("x", <f64>::TYPE_INFO);
        let field_y = Field::new("y", <f64>::TYPE_INFO);
        let fields = [&field_x, &field_y];

        let custom_type = CustomType::new("Point", &fields);
        #[cfg(feature = "std")]
        let json = serde_json::to_string(&custom_type).unwrap();
        #[cfg(feature = "embedded")]
        let json = {
            let mut buffer = [0u8; 64];
            let len = serde_json_core::to_slice(&custom_type, &mut buffer).unwrap();
            let vec = mayheap::Vec::<_, 64>::from_slice(&buffer[..len]).unwrap();
            mayheap::String::<64>::from_utf8(vec).unwrap()
        };
        assert_eq!(json, r#""type Point (x: float, y: float)""#);
    }
}
