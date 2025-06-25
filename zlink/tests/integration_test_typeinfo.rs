//! Integration test to verify end-to-end TypeInfo functionality.
//!
//! This test verifies that:
//! 1. The TypeInfo trait and derive macro are both available from zlink::idl
//! 2. They work together seamlessly
//! 3. The macro generates correct TYPE_INFO implementations
//! 4. Both simple and complex types work as expected

use zlink::idl::{Type, TypeInfo};

#[derive(TypeInfo)]
#[allow(dead_code)]
struct Person {
    name: String,
    age: u32,
    email: Option<String>,
}

#[derive(TypeInfo)]
#[allow(dead_code)]
struct Point(f64, f64, f64);

#[derive(TypeInfo)]
struct Unit;

#[derive(TypeInfo)]
#[allow(dead_code)]
struct Complex {
    id: u64,
    tags: Vec<String>,
    metadata: Option<Vec<String>>,
    active: bool,
}

#[test]
fn test_typeinfo_integration() {
    // Test that TYPE_INFO is available and returns correct type
    match Person::TYPE_INFO {
        Type::Struct(fields) => {
            let field_vec: Vec<_> = fields.iter().collect();
            assert_eq!(field_vec.len(), 3);

            assert_eq!(field_vec[0].name(), "name");
            assert_eq!(field_vec[0].ty(), &Type::String);

            assert_eq!(field_vec[1].name(), "age");
            assert_eq!(field_vec[1].ty(), &Type::Int);

            assert_eq!(field_vec[2].name(), "email");
            match field_vec[2].ty() {
                Type::Optional(inner) => assert_eq!(inner.inner(), &Type::String),
                _ => panic!("Expected optional type"),
            }
        }
        _ => panic!("Expected struct type"),
    }

    // Test tuple struct
    match Point::TYPE_INFO {
        Type::Struct(fields) => {
            let field_vec: Vec<_> = fields.iter().collect();
            assert_eq!(field_vec.len(), 3);
            assert_eq!(field_vec[0].name(), "field0");
            assert_eq!(field_vec[0].ty(), &Type::Float);
        }
        _ => panic!("Expected struct type"),
    }

    // Test unit struct
    match Unit::TYPE_INFO {
        Type::Struct(fields) => {
            assert_eq!(fields.len(), 0);
        }
        _ => panic!("Expected struct type"),
    }

    // Test complex types
    match Complex::TYPE_INFO {
        Type::Struct(fields) => {
            let field_vec: Vec<_> = fields.iter().collect();
            assert_eq!(field_vec.len(), 4);

            // Test Vec<String> field
            assert_eq!(field_vec[1].name(), "tags");
            match field_vec[1].ty() {
                Type::Array(inner) => assert_eq!(inner.inner(), &Type::String),
                _ => panic!("Expected array type"),
            }

            // Test Option<Vec<String>> field
            assert_eq!(field_vec[2].name(), "metadata");
            match field_vec[2].ty() {
                Type::Optional(optional_inner) => match optional_inner.inner() {
                    Type::Array(array_inner) => assert_eq!(array_inner.inner(), &Type::String),
                    _ => panic!("Expected array inside optional"),
                },
                _ => panic!("Expected optional type"),
            }
        }
        _ => panic!("Expected struct type"),
    }
}

#[test]
fn test_const_compatibility() {
    // Verify that TYPE_INFO can be used in const contexts
    const _PERSON_TYPE: &Type<'static> = Person::TYPE_INFO;
    const _POINT_TYPE: &Type<'static> = Point::TYPE_INFO;
    const _UNIT_TYPE: &Type<'static> = Unit::TYPE_INFO;
    const _COMPLEX_TYPE: &Type<'static> = Complex::TYPE_INFO;
}

#[test]
fn test_trait_and_macro_same_name() {
    // This test verifies that we can import both the trait and macro
    // with the same name from the same module, just like zvariant does
    use zlink::idl::TypeInfo; // This imports both trait and macro

    #[derive(TypeInfo)]
    #[allow(dead_code)]
    struct Local {
        value: i32,
    }

    // Use the trait method
    match Local::TYPE_INFO {
        Type::Struct(fields) => {
            let field_vec: Vec<_> = fields.iter().collect();
            assert_eq!(field_vec.len(), 1);
            assert_eq!(field_vec[0].name(), "value");
            assert_eq!(field_vec[0].ty(), &Type::Int);
        }
        _ => panic!("Expected struct type"),
    }
}
