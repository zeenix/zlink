//! Integration test to verify end-to-end CustomTypeInfo derive macro functionality.
//!
//! This test verifies that:
//! 1. The CustomTypeInfo derive macro is available from zlink::idl::custom
//! 2. It generates correct custom type implementations for structs and enums
//! 3. The generated types include proper names and work with the API
//! 4. Both the trait and derive macro are available from the same module

use zlink::idl::custom::{Type, TypeInfo};

#[test]
fn test_custom_typeinfo_derive_integration() {
    // Test struct with CustomTypeInfo derive
    #[derive(TypeInfo)]
    #[allow(dead_code)]
    struct Point {
        x: f64,
        y: f64,
    }

    // Verify the derive macro generated the correct implementation
    match Point::TYPE_INFO {
        Type::Object(obj) => {
            assert_eq!(obj.name(), "Point");

            let fields: Vec<_> = obj.fields().collect();
            assert_eq!(fields.len(), 2);

            assert_eq!(fields[0].name(), "x");
            assert_eq!(fields[1].name(), "y");
        }
        _ => panic!("Expected custom object type for Point"),
    }

    // Test that the type name is accessible
    assert_eq!(Point::TYPE_INFO.name(), "Point");
    assert!(Point::TYPE_INFO.is_object());
    assert!(!Point::TYPE_INFO.is_enum());
}

#[test]
fn test_custom_enum_derive_integration() {
    // Test enum with CustomTypeInfo derive
    #[derive(TypeInfo)]
    #[allow(dead_code)]
    enum Status {
        Active,
        Inactive,
        Pending,
    }

    // Verify the derive macro generated the correct implementation
    match Status::TYPE_INFO {
        Type::Enum(enm) => {
            assert_eq!(enm.name(), "Status");

            let variants: Vec<_> = enm.variants().collect();
            assert_eq!(variants.len(), 3);

            assert_eq!(*variants[0], "Active");
            assert_eq!(*variants[1], "Inactive");
            assert_eq!(*variants[2], "Pending");
        }
        _ => panic!("Expected custom enum type for Status"),
    }

    // Test that the type name is accessible
    assert_eq!(Status::TYPE_INFO.name(), "Status");
    assert!(!Status::TYPE_INFO.is_object());
    assert!(Status::TYPE_INFO.is_enum());
}

#[test]
fn test_unit_struct_derive_integration() {
    // Test unit struct
    #[derive(TypeInfo)]
    #[allow(dead_code)]
    struct Unit;

    match Unit::TYPE_INFO {
        Type::Object(obj) => {
            assert_eq!(obj.name(), "Unit");

            let fields: Vec<_> = obj.fields().collect();
            assert_eq!(fields.len(), 0);
        }
        _ => panic!("Expected custom object type for Unit"),
    }
}

#[test]
fn test_complex_struct_derive_integration() {
    // Test struct with various field types
    #[derive(TypeInfo)]
    #[allow(dead_code)]
    struct Person {
        name: String,
        age: u32,
        email: Option<String>,
        tags: Vec<String>,
        active: bool,
    }

    match Person::TYPE_INFO {
        Type::Object(obj) => {
            assert_eq!(obj.name(), "Person");

            let fields: Vec<_> = obj.fields().collect();
            assert_eq!(fields.len(), 5);

            // Verify field names
            assert_eq!(fields[0].name(), "name");
            assert_eq!(fields[1].name(), "age");
            assert_eq!(fields[2].name(), "email");
            assert_eq!(fields[3].name(), "tags");
            assert_eq!(fields[4].name(), "active");
        }
        _ => panic!("Expected custom object type for Person"),
    }
}

#[test]
fn test_const_compatibility() {
    #[derive(TypeInfo)]
    #[allow(dead_code)]
    struct TestStruct {
        value: i32,
    }

    #[derive(TypeInfo)]
    #[allow(dead_code)]
    enum TestEnum {
        Variant1,
        Variant2,
    }

    // Verify that TYPE_INFO can be used in const contexts
    const _STRUCT_TYPE: &Type<'static> = TestStruct::TYPE_INFO;
    const _ENUM_TYPE: &Type<'static> = TestEnum::TYPE_INFO;
}

#[test]
fn test_trait_and_derive_same_import() {
    // This test verifies that we can import both the trait and derive macro
    // with the same name from the custom module, just like the regular TypeInfo
    use zlink::idl::custom::TypeInfo; // This imports both trait and derive macro

    #[derive(TypeInfo)]
    #[allow(dead_code)]
    struct LocalType {
        field: String,
    }

    // Verify we can use the trait method
    let type_info = LocalType::TYPE_INFO;
    assert_eq!(type_info.name(), "LocalType");
}

#[test]
fn test_single_variant_enum() {
    #[derive(TypeInfo)]
    #[allow(dead_code)]
    enum SingleVariant {
        Only,
    }

    match SingleVariant::TYPE_INFO {
        Type::Enum(enm) => {
            assert_eq!(enm.name(), "SingleVariant");

            let variants: Vec<_> = enm.variants().collect();
            assert_eq!(variants.len(), 1);
            assert_eq!(*variants[0], "Only");
        }
        _ => panic!("Expected custom enum type for SingleVariant"),
    }
}

#[test]
fn test_type_names_preserved() {
    // Test that type names are exactly preserved as written
    #[derive(TypeInfo)]
    #[allow(dead_code, non_camel_case_types)]
    struct snake_case_name {
        value: i32,
    }

    #[derive(TypeInfo)]
    #[allow(dead_code, non_camel_case_types)]
    enum MixedCaseEnum {
        VariantOne,
        variant_two,
        VARIANT_THREE,
    }

    assert_eq!(snake_case_name::TYPE_INFO.name(), "snake_case_name");
    assert_eq!(MixedCaseEnum::TYPE_INFO.name(), "MixedCaseEnum");

    if let Type::Enum(enm) = MixedCaseEnum::TYPE_INFO {
        let variants: Vec<_> = enm.variants().collect();
        assert_eq!(*variants[0], "VariantOne");
        assert_eq!(*variants[1], "variant_two");
        assert_eq!(*variants[2], "VARIANT_THREE");
    }
}

#[test]
fn test_derive_macro_available_from_main_module() {
    // Verify the derive macro is available from the expected location
    use zlink::idl::custom::TypeInfo;

    #[derive(TypeInfo)]
    #[allow(dead_code)]
    struct ExportTest {
        data: String,
    }

    // If this compiles and we can access TYPE_INFO, the export works
    assert_eq!(ExportTest::TYPE_INFO.name(), "ExportTest");
}

#[test]
fn test_enum_variant_names_not_renamed_for_encoding() {
    // This test verifies that enum variant names in TypeInfo are preserved exactly
    // as written in the code, ignoring any serde renaming attributes
    use serde::{Deserialize, Serialize};

    #[derive(TypeInfo, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(dead_code)]
    enum DriveState {
        Idle,
        Spooling,
        Busy,
        VeryBusy, // This would be "very_busy" in JSON but should be "VeryBusy" in TypeInfo
    }

    match DriveState::TYPE_INFO {
        Type::Enum(enm) => {
            assert_eq!(enm.name(), "DriveState");

            let variants: Vec<_> = enm.variants().collect();
            assert_eq!(variants.len(), 4);

            // Verify that the original variant names are preserved, not the serde-renamed ones
            assert_eq!(*variants[0], "Idle"); // Not "idle"
            assert_eq!(*variants[1], "Spooling"); // Not "spooling"
            assert_eq!(*variants[2], "Busy"); // Not "busy"
            assert_eq!(*variants[3], "VeryBusy"); // Not "very_busy"
        }
        _ => panic!("Expected custom enum type for DriveState"),
    }
}
