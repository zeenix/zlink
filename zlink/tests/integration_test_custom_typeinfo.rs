//! Integration test to verify end-to-end custom::TypeInfo functionality.
//!
//! This test verifies that:
//! 1. The custom::TypeInfo trait is available from zlink::idl::custom
//! 2. It can be implemented manually for custom types
//! 3. It generates correct TYPE_INFO implementations with type names
//! 4. Both object and enum custom types work as expected

use zlink::idl::{
    custom::{Enum, Object, Type, TypeInfo},
    Field, Type as VarlinkType, TypeInfo as VarlinkTypeInfo,
};

// Test struct that implements custom::TypeInfo
struct Point;

impl TypeInfo for Point {
    const TYPE_INFO: &'static Type<'static> = &{
        static FIELD_X: Field<'static> = Field::new("x", <f64 as VarlinkTypeInfo>::TYPE_INFO);
        static FIELD_Y: Field<'static> = Field::new("y", <f64 as VarlinkTypeInfo>::TYPE_INFO);
        static FIELDS: &[&Field<'static>] = &[&FIELD_X, &FIELD_Y];

        Type::Object(Object::new("Point", FIELDS))
    };
}

// Test enum that implements custom::TypeInfo
struct Color;

impl TypeInfo for Color {
    const TYPE_INFO: &'static Type<'static> = &{
        static VARIANT_RED: &str = "Red";
        static VARIANT_GREEN: &str = "Green";
        static VARIANT_BLUE: &str = "Blue";
        static VARIANTS: &[&'static &'static str] = &[&VARIANT_RED, &VARIANT_GREEN, &VARIANT_BLUE];

        Type::Enum(Enum::new("Color", VARIANTS))
    };
}

// Complex struct with various field types
struct Person;

impl TypeInfo for Person {
    const TYPE_INFO: &'static Type<'static> = &{
        static FIELD_NAME: Field<'static> =
            Field::new("name", <&str as VarlinkTypeInfo>::TYPE_INFO);
        static FIELD_AGE: Field<'static> = Field::new("age", <u32 as VarlinkTypeInfo>::TYPE_INFO);
        static FIELD_ACTIVE: Field<'static> =
            Field::new("active", <bool as VarlinkTypeInfo>::TYPE_INFO);
        static FIELDS: &[&Field<'static>] = &[&FIELD_NAME, &FIELD_AGE, &FIELD_ACTIVE];

        Type::Object(Object::new("Person", FIELDS))
    };
}

#[test]
fn test_custom_struct_typeinfo_integration() {
    // Test that TYPE_INFO is available and returns correct custom type
    match Point::TYPE_INFO {
        Type::Object(obj) => {
            assert_eq!(obj.name(), "Point");

            let fields: Vec<_> = obj.fields().collect();
            assert_eq!(fields.len(), 2);

            assert_eq!(fields[0].name(), "x");
            assert_eq!(fields[0].ty(), &VarlinkType::Float);

            assert_eq!(fields[1].name(), "y");
            assert_eq!(fields[1].ty(), &VarlinkType::Float);
        }
        _ => panic!("Expected custom object type"),
    }

    // Verify the type has a name (unlike regular TypeInfo)
    assert_eq!(Point::TYPE_INFO.name(), "Point");
    assert!(Point::TYPE_INFO.is_object());
    assert!(!Point::TYPE_INFO.is_enum());
}

#[test]
fn test_custom_enum_typeinfo_integration() {
    // Test enum custom type
    match Color::TYPE_INFO {
        Type::Enum(enm) => {
            assert_eq!(enm.name(), "Color");

            let variants: Vec<_> = enm.variants().collect();
            assert_eq!(variants.len(), 3);

            assert_eq!(*variants[0], "Red");
            assert_eq!(*variants[1], "Green");
            assert_eq!(*variants[2], "Blue");
        }
        _ => panic!("Expected custom enum type"),
    }

    // Verify the type has a name and correct variant type
    assert_eq!(Color::TYPE_INFO.name(), "Color");
    assert!(!Color::TYPE_INFO.is_object());
    assert!(Color::TYPE_INFO.is_enum());
}

#[test]
fn test_complex_custom_typeinfo_integration() {
    // Test complex struct with multiple field types
    match Person::TYPE_INFO {
        Type::Object(obj) => {
            assert_eq!(obj.name(), "Person");

            let fields: Vec<_> = obj.fields().collect();
            assert_eq!(fields.len(), 3);

            assert_eq!(fields[0].name(), "name");
            assert_eq!(fields[0].ty(), &VarlinkType::String);

            assert_eq!(fields[1].name(), "age");
            assert_eq!(fields[1].ty(), &VarlinkType::Int);

            assert_eq!(fields[2].name(), "active");
            assert_eq!(fields[2].ty(), &VarlinkType::Bool);
        }
        _ => panic!("Expected custom object type"),
    }
}

#[test]
fn test_const_compatibility() {
    // Verify that TYPE_INFO can be used in const contexts
    const _POINT_TYPE: &Type<'static> = Point::TYPE_INFO;
    const _COLOR_TYPE: &Type<'static> = Color::TYPE_INFO;
    const _PERSON_TYPE: &Type<'static> = Person::TYPE_INFO;
}

#[test]
fn test_trait_imports() {
    // This test verifies that we can import the custom TypeInfo trait
    // and it doesn't conflict with the regular TypeInfo trait
    use zlink::idl::{custom::TypeInfo as CustomTypeInfo, TypeInfo as RegularTypeInfo};

    // Both traits should be available and distinct
    const _CUSTOM: &Type<'static> = Point::TYPE_INFO;
    const _REGULAR: &VarlinkType<'static> = <i32 as RegularTypeInfo>::TYPE_INFO;
}

#[test]
fn test_type_accessor_methods() {
    // Test the convenience methods on custom types
    let point_type = Point::TYPE_INFO;
    assert!(point_type.is_object());
    assert!(!point_type.is_enum());
    assert!(point_type.as_object().is_some());
    assert!(point_type.as_enum().is_none());

    let color_type = Color::TYPE_INFO;
    assert!(!color_type.is_object());
    assert!(color_type.is_enum());
    assert!(color_type.as_object().is_none());
    assert!(color_type.as_enum().is_some());
}
