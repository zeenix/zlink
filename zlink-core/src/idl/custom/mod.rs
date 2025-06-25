//! Custom type definitions for Varlink IDL.
//!
//! This module contains definitions for custom types in Varlink IDL, including
//! object types (struct-like with named fields) and enum types (with named variants).

mod object;
pub use object::Object;

mod r#enum;
pub use r#enum::Enum;

mod r#type;
pub use r#type::Type;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::idl::{Field, Type as VarlinkType, TypeInfo};

    #[test]
    fn object_creation() {
        let field_x = Field::new("x", <f64>::TYPE_INFO);
        let field_y = Field::new("y", <f64>::TYPE_INFO);
        let fields = [&field_x, &field_y];

        let custom_obj = Object::new("Point", &fields);
        assert_eq!(custom_obj.name(), "Point");
        assert_eq!(custom_obj.fields().count(), 2);

        // Check the fields individually - order and values.
        let fields = custom_obj.fields().collect::<mayheap::Vec<_, 8>>();
        assert_eq!(fields[0].name(), "x");
        assert_eq!(fields[0].ty(), &VarlinkType::Float);
        assert_eq!(fields[1].name(), "y");
        assert_eq!(fields[1].ty(), &VarlinkType::Float);
    }

    #[test]
    fn enum_creation() {
        let custom_enum = Enum::new("Color", &[&"red", &"green", &"blue"]);

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
        let field_x = Field::new("x", <f64>::TYPE_INFO);
        let field_y = Field::new("y", <f64>::TYPE_INFO);
        let fields = [&field_x, &field_y];

        let custom_obj = Object::new("Point", &fields);
        let custom_type = Type::from(custom_obj);

        assert_eq!(custom_type.name(), "Point");
        assert!(custom_type.is_object());
        assert!(!custom_type.is_enum());
        assert!(custom_type.as_object().is_some());
        assert!(custom_type.as_enum().is_none());
    }

    #[test]
    fn type_from_enum() {
        let custom_enum = Enum::new("Color", &[&"red", &"green", &"blue"]);
        let custom_type = Type::from(custom_enum);

        assert_eq!(custom_type.name(), "Color");
        assert!(!custom_type.is_object());
        assert!(custom_type.is_enum());
        assert!(custom_type.as_object().is_none());
        assert!(custom_type.as_enum().is_some());
    }

    #[test]
    fn object_serialization() {
        let field_x = Field::new("x", <f64>::TYPE_INFO);
        let field_y = Field::new("y", <f64>::TYPE_INFO);
        let fields = [&field_x, &field_y];

        let custom_obj = Object::new("Point", &fields);
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
        let custom_enum = Enum::new("Status", &[&"active", &"inactive", &"pending"]);

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
        let field_x = Field::new("x", <i64>::TYPE_INFO);
        let field_y = Field::new("y", <i64>::TYPE_INFO);
        let fields = [&field_x, &field_y];
        let custom_obj = Object::new("Point", &fields);
        let custom_type = Type::from(custom_obj);
        use core::fmt::Write;
        let mut buf = mayheap::String::<64>::new();
        write!(buf, "{}", custom_type).unwrap();
        assert_eq!(buf.as_str(), "type Point (x: int, y: int)");

        // Test enum display
        let custom_enum = Enum::new("Direction", &[&"north", &"south", &"east", &"west"]);
        let custom_type = Type::from(custom_enum);
        let mut buf = mayheap::String::<128>::new();
        write!(buf, "{}", custom_type).unwrap();
        assert_eq!(buf.as_str(), "type Direction (north, south, east, west)");
    }

    #[cfg(feature = "std")]
    #[test]
    fn owned_types() {
        // Test owned object
        let fields = vec![
            Field::new("name", <&str>::TYPE_INFO),
            Field::new("age", <i64>::TYPE_INFO),
        ];
        let custom_obj = Object::new_owned("Person", fields);
        assert_eq!(custom_obj.name(), "Person");
        use core::fmt::Write;
        let mut buf = mayheap::String::<64>::new();
        write!(buf, "{}", custom_obj).unwrap();
        assert_eq!(buf.as_str(), "type Person (name: string, age: int)");

        // Test owned enum
        let custom_enum = Enum::new_owned("Size", vec!["small", "medium", "large"]);
        assert_eq!(custom_enum.name(), "Size");
        let mut buf = mayheap::String::<64>::new();
        write!(buf, "{}", custom_enum).unwrap();
        assert_eq!(buf.as_str(), "type Size (small, medium, large)");
    }
}
