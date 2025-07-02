//! Type definition for Varlink IDL custom types.

use core::fmt;

use serde::Serialize;

#[cfg(feature = "idl-parse")]
use serde::Deserialize;

use super::{CustomEnum, CustomObject};

/// A custom type definition in Varlink IDL.
///
/// This can be either a struct-like object type with named fields,
/// or an enum-like type with named variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomType<'a> {
    /// A struct-like custom type with named fields.
    Object(CustomObject<'a>),
    /// An enum-like custom type with named variants.
    Enum(CustomEnum<'a>),
}

impl<'a> CustomType<'a> {
    /// Returns the name of the custom type.
    pub fn name(&self) -> &'a str {
        match self {
            CustomType::Object(obj) => obj.name(),
            CustomType::Enum(enm) => enm.name(),
        }
    }

    /// Returns true if this is an object custom type.
    pub fn is_object(&self) -> bool {
        matches!(self, CustomType::Object(_))
    }

    /// Returns true if this is an enum custom type.
    pub fn is_enum(&self) -> bool {
        matches!(self, CustomType::Enum(_))
    }

    /// Returns the object if this is an object custom type.
    pub fn as_object(&self) -> Option<&CustomObject<'a>> {
        match self {
            CustomType::Object(obj) => Some(obj),
            CustomType::Enum(_) => None,
        }
    }

    /// Returns the enum if this is an enum custom type.
    pub fn as_enum(&self) -> Option<&CustomEnum<'a>> {
        match self {
            CustomType::Object(_) => None,
            CustomType::Enum(enm) => Some(enm),
        }
    }
}

impl<'a> From<CustomObject<'a>> for CustomType<'a> {
    fn from(obj: CustomObject<'a>) -> Self {
        CustomType::Object(obj)
    }
}

impl<'a> From<CustomEnum<'a>> for CustomType<'a> {
    fn from(enm: CustomEnum<'a>) -> Self {
        CustomType::Enum(enm)
    }
}

impl<'a> fmt::Display for CustomType<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CustomType::Object(obj) => write!(f, "{obj}"),
            CustomType::Enum(enm) => write!(f, "{enm}"),
        }
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
    use crate::{
        idl::{self, Field},
        introspect,
    };

    #[test]
    fn object_creation() {
        let field_x = Field::new("x", <f64 as introspect::Type>::TYPE, &[]);
        let field_y = Field::new("y", <f64 as introspect::Type>::TYPE, &[]);
        let fields = [&field_x, &field_y];

        let custom_obj = CustomObject::new("Point", &fields, &[]);
        assert_eq!(custom_obj.name(), "Point");
        assert_eq!(custom_obj.fields().count(), 2);

        // Check the fields individually - order and values.
        let fields = custom_obj.fields().collect::<mayheap::Vec<_, 8>>();
        assert_eq!(fields[0].name(), "x");
        assert_eq!(fields[0].ty(), &idl::Type::Float);
        assert_eq!(fields[1].name(), "y");
        assert_eq!(fields[1].ty(), &idl::Type::Float);
    }

    #[test]
    fn enum_creation() {
        let custom_enum = CustomEnum::new("Color", &[&"red", &"green", &"blue"], &[]);

        assert_eq!(custom_enum.name(), "Color");
        assert_eq!(custom_enum.variants().count(), 3);

        // Check the variants individually - order and values.
        let variants = custom_enum.variants().collect::<mayheap::Vec<_, 8>>();
        assert_eq!(*variants[0], "red");
        assert_eq!(*variants[1], "green");
        assert_eq!(*variants[2], "blue");
    }

    #[test]
    fn type_from_object() {
        let field_x = Field::new("x", <f64 as introspect::Type>::TYPE, &[]);
        let field_y = Field::new("y", <f64 as introspect::Type>::TYPE, &[]);
        let fields = [&field_x, &field_y];

        let custom_obj = CustomObject::new("Point", &fields, &[]);
        let custom_type = idl::CustomType::from(custom_obj);

        assert_eq!(custom_type.name(), "Point");
        assert!(custom_type.is_object());
        assert!(!custom_type.is_enum());
        assert!(custom_type.as_object().is_some());
        assert!(custom_type.as_enum().is_none());
    }

    #[test]
    fn type_from_enum() {
        let custom_enum = CustomEnum::new("Color", &[&"red", &"green", &"blue"], &[]);
        let custom_type = idl::CustomType::from(custom_enum);

        assert_eq!(custom_type.name(), "Color");
        assert!(!custom_type.is_object());
        assert!(custom_type.is_enum());
        assert!(custom_type.as_object().is_none());
        assert!(custom_type.as_enum().is_some());
    }

    #[test]
    fn object_serialization() {
        let field_x = Field::new("x", <f64 as introspect::Type>::TYPE, &[]);
        let field_y = Field::new("y", <f64 as introspect::Type>::TYPE, &[]);
        let fields = [&field_x, &field_y];

        let custom_obj = CustomObject::new("Point", &fields, &[]);
        #[cfg(feature = "std")]
        let json = serde_json::to_string(&custom_obj).unwrap();
        #[cfg(feature = "embedded")]
        let json = {
            let mut buffer = [0u8; 64];
            let len = serde_json_core::to_slice(&custom_obj, &mut buffer).unwrap();
            let vec = mayheap::Vec::<_, 64>::from_slice(&buffer[..len]).unwrap();
            mayheap::String::<64>::from_utf8(vec).unwrap()
        };
        assert_eq!(json, r#""type Point (x: float, y: float)""#);
    }

    #[test]
    fn enum_serialization() {
        let custom_enum = CustomEnum::new("Status", &[&"active", &"inactive", &"pending"], &[]);

        #[cfg(feature = "std")]
        let json = serde_json::to_string(&custom_enum).unwrap();
        #[cfg(feature = "embedded")]
        let json = {
            let mut buffer = [0u8; 64];
            let len = serde_json_core::to_slice(&custom_enum, &mut buffer).unwrap();
            let vec = mayheap::Vec::<_, 64>::from_slice(&buffer[..len]).unwrap();
            mayheap::String::<64>::from_utf8(vec).unwrap()
        };
        assert_eq!(json, r#""type Status (active, inactive, pending)""#);
    }

    #[test]
    fn type_display() {
        // Test object display
        let field_x = Field::new("x", <i64 as introspect::Type>::TYPE, &[]);
        let field_y = Field::new("y", <i64 as introspect::Type>::TYPE, &[]);
        let fields = [&field_x, &field_y];
        let custom_obj = CustomObject::new("Point", &fields, &[]);
        let custom_type = idl::CustomType::from(custom_obj);
        use core::fmt::Write;
        let mut buf = mayheap::String::<64>::new();
        write!(buf, "{}", custom_type).unwrap();
        assert_eq!(buf.as_str(), "type Point (x: int, y: int)");

        // Test enum display
        let custom_enum =
            CustomEnum::new("Direction", &[&"north", &"south", &"east", &"west"], &[]);
        let custom_type = idl::CustomType::from(custom_enum);
        let mut buf = mayheap::String::<128>::new();
        write!(buf, "{}", custom_type).unwrap();
        assert_eq!(buf.as_str(), "type Direction (north, south, east, west)");
    }

    #[cfg(feature = "std")]
    #[test]
    fn owned_types() {
        // Test owned object
        let fields = vec![
            Field::new("name", <&str as introspect::Type>::TYPE, &[]),
            Field::new("age", <i64 as introspect::Type>::TYPE, &[]),
        ];
        let custom_obj = CustomObject::new_owned("Person", fields, vec![]);
        assert_eq!(custom_obj.name(), "Person");
        use core::fmt::Write;
        let mut buf = mayheap::String::<64>::new();
        write!(buf, "{}", custom_obj).unwrap();
        assert_eq!(buf.as_str(), "type Person (name: string, age: int)");

        // Test owned enum
        let custom_enum = CustomEnum::new_owned("Size", vec!["small", "medium", "large"], vec![]);
        assert_eq!(custom_enum.name(), "Size");
        let mut buf = mayheap::String::<64>::new();
        write!(buf, "{}", custom_enum).unwrap();
        assert_eq!(buf.as_str(), "type Size (small, medium, large)");
    }
}
