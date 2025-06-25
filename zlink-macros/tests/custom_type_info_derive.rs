use zlink::idl::{
    custom::{Type, TypeInfo},
    Type as VarlinkType, TypeInfo as RegularTypeInfo,
};

#[test]
fn named_struct_custom_type_info() {
    match Person::TYPE_INFO {
        Type::Object(obj) => {
            // Verify the custom type includes the name
            assert_eq!(obj.name(), "Person");

            let field_vec: Vec<_> = obj.fields().collect();
            assert_eq!(field_vec.len(), 3);

            // Check name field
            assert_eq!(field_vec[0].name(), "name");
            assert_eq!(field_vec[0].ty(), &VarlinkType::String);

            // Check age field
            assert_eq!(field_vec[1].name(), "age");
            assert_eq!(field_vec[1].ty(), &VarlinkType::Int);

            // Check active field
            assert_eq!(field_vec[2].name(), "active");
            assert_eq!(field_vec[2].ty(), &VarlinkType::Bool);
        }
        _ => panic!("Expected custom object type for Person"),
    }
}

#[test]
fn unit_struct_custom_type_info() {
    match Unit::TYPE_INFO {
        Type::Object(obj) => {
            // Verify the custom type includes the name
            assert_eq!(obj.name(), "Unit");

            let field_vec: Vec<_> = obj.fields().collect();
            assert_eq!(field_vec.len(), 0);
        }
        _ => panic!("Expected custom object type for Unit"),
    }
}

#[test]
fn complex_struct_custom_type_info() {
    match Complex::TYPE_INFO {
        Type::Object(obj) => {
            // Verify the custom type includes the name
            assert_eq!(obj.name(), "Complex");

            let field_vec: Vec<_> = obj.fields().collect();
            assert_eq!(field_vec.len(), 4);

            // Check id field
            assert_eq!(field_vec[0].name(), "id");
            assert_eq!(field_vec[0].ty(), &VarlinkType::Int);

            // Check description field (Option<String>)
            assert_eq!(field_vec[1].name(), "description");
            match field_vec[1].ty() {
                VarlinkType::Optional(inner) => assert_eq!(inner.inner(), &VarlinkType::String),
                _ => panic!("Expected optional type for description"),
            }

            // Check tags field (Vec<String>)
            assert_eq!(field_vec[2].name(), "tags");
            match field_vec[2].ty() {
                VarlinkType::Array(inner) => assert_eq!(inner.inner(), &VarlinkType::String),
                _ => panic!("Expected array type for tags"),
            }

            // Check coordinates field (Option<Vec<f64>>)
            assert_eq!(field_vec[3].name(), "coordinates");
            match field_vec[3].ty() {
                VarlinkType::Optional(optional_inner) => match optional_inner.inner() {
                    VarlinkType::Array(array_inner) => {
                        assert_eq!(array_inner.inner(), &VarlinkType::Float)
                    }
                    _ => panic!("Expected array inside optional for coordinates"),
                },
                _ => panic!("Expected optional type for coordinates"),
            }
        }
        _ => panic!("Expected custom object type for Complex"),
    }
}

#[test]
fn primitives_struct_custom_type_info() {
    match Primitives::TYPE_INFO {
        Type::Object(obj) => {
            // Verify the custom type includes the name
            assert_eq!(obj.name(), "Primitives");

            let field_vec: Vec<_> = obj.fields().collect();
            assert_eq!(field_vec.len(), 5);

            assert_eq!(field_vec[0].name(), "boolean");
            assert_eq!(field_vec[0].ty(), &VarlinkType::Bool);

            assert_eq!(field_vec[1].name(), "signed");
            assert_eq!(field_vec[1].ty(), &VarlinkType::Int);

            assert_eq!(field_vec[2].name(), "unsigned");
            assert_eq!(field_vec[2].ty(), &VarlinkType::Int);

            assert_eq!(field_vec[3].name(), "floating");
            assert_eq!(field_vec[3].ty(), &VarlinkType::Float);

            assert_eq!(field_vec[4].name(), "text");
            assert_eq!(field_vec[4].ty(), &VarlinkType::String);
        }
        _ => panic!("Expected custom object type for Primitives"),
    }
}

#[test]
fn basic_enum_custom_type_info() {
    match Status::TYPE_INFO {
        Type::Enum(enm) => {
            // Verify the custom type includes the name
            assert_eq!(enm.name(), "Status");

            let variant_vec: Vec<_> = enm.variants().collect();
            assert_eq!(variant_vec.len(), 3);
            assert_eq!(*variant_vec[0], "Active");
            assert_eq!(*variant_vec[1], "Inactive");
            assert_eq!(*variant_vec[2], "Pending");
        }
        _ => panic!("Expected custom enum type for Status"),
    }
}

#[test]
fn multi_variant_enum_custom_type_info() {
    match Color::TYPE_INFO {
        Type::Enum(enm) => {
            // Verify the custom type includes the name
            assert_eq!(enm.name(), "Color");

            let variant_vec: Vec<_> = enm.variants().collect();
            assert_eq!(variant_vec.len(), 6);
            assert_eq!(*variant_vec[0], "Red");
            assert_eq!(*variant_vec[1], "Green");
            assert_eq!(*variant_vec[2], "Blue");
            assert_eq!(*variant_vec[3], "Yellow");
            assert_eq!(*variant_vec[4], "Orange");
            assert_eq!(*variant_vec[5], "Purple");
        }
        _ => panic!("Expected custom enum type for Color"),
    }
}

#[test]
fn single_variant_enum_custom_type_info() {
    match UnitEnum::TYPE_INFO {
        Type::Enum(enm) => {
            // Verify the custom type includes the name
            assert_eq!(enm.name(), "UnitEnum");

            let variant_vec: Vec<_> = enm.variants().collect();
            assert_eq!(variant_vec.len(), 1);
            assert_eq!(*variant_vec[0], "Only");
        }
        _ => panic!("Expected custom enum type for UnitEnum"),
    }
}

// Test that the macro generates const-compatible code
#[test]
fn const_compatibility() {
    const _: &Type<'static> = Person::TYPE_INFO;
    const _: &Type<'static> = Unit::TYPE_INFO;
    const _: &Type<'static> = Complex::TYPE_INFO;
    const _: &Type<'static> = Status::TYPE_INFO;
    const _: &Type<'static> = Color::TYPE_INFO;
    const _: &Type<'static> = UnitEnum::TYPE_INFO;
}

#[test]
fn nested_struct_custom_type_info() {
    // First verify Address works
    match <Address as TypeInfo>::TYPE_INFO {
        Type::Object(obj) => {
            assert_eq!(obj.name(), "Address");
            let field_vec: Vec<_> = obj.fields().collect();
            assert_eq!(field_vec.len(), 2);
        }
        _ => panic!("Expected custom object type for Address"),
    }

    // Then verify PersonWithAddress works
    match <PersonWithAddress as TypeInfo>::TYPE_INFO {
        Type::Object(obj) => {
            assert_eq!(obj.name(), "PersonWithAddress");
            let field_vec: Vec<_> = obj.fields().collect();
            assert_eq!(field_vec.len(), 2);

            assert_eq!(field_vec[0].name(), "name");
            assert_eq!(field_vec[0].ty(), &VarlinkType::String);

            assert_eq!(field_vec[1].name(), "address");
            // The address field should reference Address::TYPE_INFO through regular TypeInfo
            // This tests that custom TypeInfo can reference regular TypeInfo for field types
            match field_vec[1].ty() {
                VarlinkType::Object(_) => {
                    // This should be a regular inline type, not a custom named type
                    // Custom TypeInfo uses regular TypeInfo for field types
                }
                _ => panic!("Expected object type for address field"),
            }
        }
        _ => panic!("Expected custom object type for PersonWithAddress"),
    }
}

#[test]
fn custom_type_name_access() {
    // Test the name() method on custom types
    assert_eq!(Person::TYPE_INFO.name(), "Person");
    assert_eq!(Unit::TYPE_INFO.name(), "Unit");
    assert_eq!(Complex::TYPE_INFO.name(), "Complex");
    assert_eq!(Status::TYPE_INFO.name(), "Status");
    assert_eq!(Color::TYPE_INFO.name(), "Color");
    assert_eq!(UnitEnum::TYPE_INFO.name(), "UnitEnum");
}

#[test]
fn custom_type_variant_checking() {
    // Test struct types
    assert!(Person::TYPE_INFO.is_object());
    assert!(!Person::TYPE_INFO.is_enum());
    assert!(Person::TYPE_INFO.as_object().is_some());
    assert!(Person::TYPE_INFO.as_enum().is_none());

    // Test enum types
    assert!(!Status::TYPE_INFO.is_object());
    assert!(Status::TYPE_INFO.is_enum());
    assert!(Status::TYPE_INFO.as_object().is_none());
    assert!(Status::TYPE_INFO.as_enum().is_some());
}

// Test basic named struct
#[derive(TypeInfo)]
#[allow(unused)]
struct Person {
    name: String,
    age: i32,
    active: bool,
}

// Test unit struct
#[derive(TypeInfo)]
#[allow(unused)]
struct Unit;

// Test struct with optional and array types
#[derive(TypeInfo)]
#[allow(unused)]
struct Complex {
    id: u64,
    description: Option<String>,
    tags: Vec<String>,
    coordinates: Option<Vec<f64>>,
}

// Test struct with primitive types
#[derive(TypeInfo)]
#[allow(unused)]
struct Primitives {
    boolean: bool,
    signed: i64,
    unsigned: u32,
    floating: f64,
    text: String,
}

// Test nested struct (this will require the other struct to also have TypeInfo)
#[derive(TypeInfo)]
#[allow(unused)]
struct PersonWithAddress {
    name: String,
    address: Address,
}

#[derive(TypeInfo, RegularTypeInfo)]
#[allow(unused)]
struct Address {
    street: String,
    city: String,
}

// Test basic unit enum
#[derive(TypeInfo)]
#[allow(unused)]
enum Status {
    Active,
    Inactive,
    Pending,
}

// Test enum with more variants
#[derive(TypeInfo)]
#[allow(unused)]
enum Color {
    Red,
    Green,
    Blue,
    Yellow,
    Orange,
    Purple,
}

// Test single variant enum
#[derive(TypeInfo)]
#[allow(unused)]
enum UnitEnum {
    Only,
}
