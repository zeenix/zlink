use zlink::idl::{Type, TypeInfo};

// Force diagnostics refresh
#[test]
fn named_struct_type_info() {
    match Person::TYPE_INFO {
        Type::Object(fields) => {
            let field_vec: Vec<_> = fields.iter().collect();
            assert_eq!(field_vec.len(), 3);

            // Check name field
            assert_eq!(field_vec[0].name(), "name");
            assert_eq!(field_vec[0].ty(), &Type::String);

            // Check age field
            assert_eq!(field_vec[1].name(), "age");
            assert_eq!(field_vec[1].ty(), &Type::Int);

            // Check active field
            assert_eq!(field_vec[2].name(), "active");
            assert_eq!(field_vec[2].ty(), &Type::Bool);
        }
        _ => panic!("Expected struct type for Person"),
    }
}

#[test]
fn unit_struct_type_info() {
    match Unit::TYPE_INFO {
        Type::Object(fields) => {
            let field_vec: Vec<_> = fields.iter().collect();
            assert_eq!(field_vec.len(), 0);
        }
        _ => panic!("Expected struct type for Unit"),
    }
}

#[test]
fn complex_struct_type_info() {
    match Complex::TYPE_INFO {
        Type::Object(fields) => {
            let field_vec: Vec<_> = fields.iter().collect();
            assert_eq!(field_vec.len(), 4);

            // Check id field
            assert_eq!(field_vec[0].name(), "id");
            assert_eq!(field_vec[0].ty(), &Type::Int);

            // Check description field (Option<String>)
            assert_eq!(field_vec[1].name(), "description");
            match field_vec[1].ty() {
                Type::Optional(inner) => assert_eq!(inner.inner(), &Type::String),
                _ => panic!("Expected optional type for description"),
            }

            // Check tags field (Vec<String>)
            assert_eq!(field_vec[2].name(), "tags");
            match field_vec[2].ty() {
                Type::Array(inner) => assert_eq!(inner.inner(), &Type::String),
                _ => panic!("Expected array type for tags"),
            }

            // Check coordinates field (Option<Vec<f64>>)
            assert_eq!(field_vec[3].name(), "coordinates");
            match field_vec[3].ty() {
                Type::Optional(optional_inner) => match optional_inner.inner() {
                    Type::Array(array_inner) => assert_eq!(array_inner.inner(), &Type::Float),
                    _ => panic!("Expected array inside optional for coordinates"),
                },
                _ => panic!("Expected optional type for coordinates"),
            }
        }
        _ => panic!("Expected struct type for Complex"),
    }
}

#[test]
fn primitives_struct_type_info() {
    match Primitives::TYPE_INFO {
        Type::Object(fields) => {
            let field_vec: Vec<_> = fields.iter().collect();
            assert_eq!(field_vec.len(), 5);

            assert_eq!(field_vec[0].name(), "boolean");
            assert_eq!(field_vec[0].ty(), &Type::Bool);

            assert_eq!(field_vec[1].name(), "signed");
            assert_eq!(field_vec[1].ty(), &Type::Int);

            assert_eq!(field_vec[2].name(), "unsigned");
            assert_eq!(field_vec[2].ty(), &Type::Int);

            assert_eq!(field_vec[3].name(), "floating");
            assert_eq!(field_vec[3].ty(), &Type::Float);

            assert_eq!(field_vec[4].name(), "text");
            assert_eq!(field_vec[4].ty(), &Type::String);
        }
        _ => panic!("Expected struct type for Primitives"),
    }
}

#[test]
fn basic_enum_type_info() {
    match Status::TYPE_INFO {
        Type::Enum(variants) => {
            let variant_vec: Vec<_> = variants.iter().collect();
            assert_eq!(variant_vec.len(), 3);
            assert_eq!(*variant_vec[0], "Active");
            assert_eq!(*variant_vec[1], "Inactive");
            assert_eq!(*variant_vec[2], "Pending");
        }
        _ => panic!("Expected enum type for Status"),
    }
}

#[test]
fn multi_variant_enum_type_info() {
    match Color::TYPE_INFO {
        Type::Enum(variants) => {
            let variant_vec: Vec<_> = variants.iter().collect();
            assert_eq!(variant_vec.len(), 6);
            assert_eq!(*variant_vec[0], "Red");
            assert_eq!(*variant_vec[1], "Green");
            assert_eq!(*variant_vec[2], "Blue");
            assert_eq!(*variant_vec[3], "Yellow");
            assert_eq!(*variant_vec[4], "Orange");
            assert_eq!(*variant_vec[5], "Purple");
        }
        _ => panic!("Expected enum type for Color"),
    }
}

#[test]
fn single_variant_enum_type_info() {
    match UnitEnum::TYPE_INFO {
        Type::Enum(variants) => {
            let variant_vec: Vec<_> = variants.iter().collect();
            assert_eq!(variant_vec.len(), 1);
            assert_eq!(*variant_vec[0], "Only");
        }
        _ => panic!("Expected enum type for UnitEnum"),
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
fn nested_struct_type_info() {
    // First verify Address works
    match Address::TYPE_INFO {
        Type::Object(fields) => {
            let field_vec: Vec<_> = fields.iter().collect();
            assert_eq!(field_vec.len(), 2);
        }
        _ => panic!("Expected struct type for Address"),
    }

    // Then verify PersonWithAddress works
    match PersonWithAddress::TYPE_INFO {
        Type::Object(fields) => {
            let field_vec: Vec<_> = fields.iter().collect();
            assert_eq!(field_vec.len(), 2);

            assert_eq!(field_vec[0].name(), "name");
            assert_eq!(field_vec[0].ty(), &Type::String);

            assert_eq!(field_vec[1].name(), "address");
            // The address field should reference Address::TYPE_INFO
            match field_vec[1].ty() {
                Type::Object(_) => {
                    // This should be the same as Address::TYPE_INFO
                    assert_eq!(field_vec[1].ty(), Address::TYPE_INFO);
                }
                _ => panic!("Expected struct type for address field"),
            }
        }
        _ => panic!("Expected struct type for PersonWithAddress"),
    }
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

#[derive(TypeInfo)]
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
