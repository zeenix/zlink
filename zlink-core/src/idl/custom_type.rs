//! Custom type, field and parameter definitions for Varlink IDL.

use core::fmt;

use serde::Serialize;

#[cfg(feature = "idl-parse")]
use serde::Deserialize;

use super::{Field, List};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::idl::{Type, TypeInfo};

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
