//! Tests for TypeInfo derive macro error handling and edge cases.

use zlink::idl::{Type, TypeInfo};

#[test]
fn test_unit_enum_works() {
    #[derive(TypeInfo)]
    #[allow(dead_code)]
    enum SimpleEnum {
        First,
        Second,
        Third,
    }

    match SimpleEnum::TYPE_INFO {
        Type::Enum(variants) => {
            let variant_vec: Vec<_> = variants.iter().collect();
            assert_eq!(variant_vec.len(), 3);
            assert_eq!(*variant_vec[0], "First");
            assert_eq!(*variant_vec[1], "Second");
            assert_eq!(*variant_vec[2], "Third");
        }
        _ => panic!("Expected enum type for SimpleEnum"),
    }
}

#[test]
fn test_single_variant_enum_works() {
    #[derive(TypeInfo)]
    #[allow(dead_code)]
    enum SingleVariant {
        Only,
    }

    match SingleVariant::TYPE_INFO {
        Type::Enum(variants) => {
            let variant_vec: Vec<_> = variants.iter().collect();
            assert_eq!(variant_vec.len(), 1);
            assert_eq!(*variant_vec[0], "Only");
        }
        _ => panic!("Expected enum type for SingleVariant"),
    }
}

// The following would cause compile-time errors if uncommented:
//
// #[derive(TypeInfo)]
// enum TupleVariantEnum {
//     Active(String),  // Error: TypeInfo derive macro only supports unit enum variants, not tuple
// variants     Inactive,
// }
//
// #[derive(TypeInfo)]
// enum StructVariantEnum {
//     Active { status: String },  // Error: TypeInfo derive macro only supports unit enum variants,
// not struct variants     Inactive,
// }
//
// #[derive(TypeInfo)]
// enum MixedEnum {
//     Unit,
//     Tuple(i32),  // Error: TypeInfo derive macro only supports unit enum variants, not tuple
// variants     Struct { field: String },  // Error: TypeInfo derive macro only supports unit enum
// variants, not struct variants }

#[test]
fn test_enum_with_many_variants() {
    #[derive(TypeInfo)]
    #[allow(dead_code)]
    enum ManyVariants {
        A,
        B,
        C,
        D,
        E,
        F,
        G,
        H,
        I,
        J,
    }

    match ManyVariants::TYPE_INFO {
        Type::Enum(variants) => {
            let variant_vec: Vec<_> = variants.iter().collect();
            assert_eq!(variant_vec.len(), 10);
            assert_eq!(*variant_vec[0], "A");
            assert_eq!(*variant_vec[9], "J");
        }
        _ => panic!("Expected enum type for ManyVariants"),
    }
}

#[test]
fn test_enum_with_unusual_names() {
    #[derive(TypeInfo)]
    #[allow(dead_code)]
    #[allow(non_camel_case_types)]
    enum UnusualNames {
        CamelCase,
        snake_case,
        UPPER_CASE,
        Mixed_Case_123,
    }

    match UnusualNames::TYPE_INFO {
        Type::Enum(variants) => {
            let variant_vec: Vec<_> = variants.iter().collect();
            assert_eq!(variant_vec.len(), 4);
            assert_eq!(*variant_vec[0], "CamelCase");
            assert_eq!(*variant_vec[1], "snake_case");
            assert_eq!(*variant_vec[2], "UPPER_CASE");
            assert_eq!(*variant_vec[3], "Mixed_Case_123");
        }
        _ => panic!("Expected enum type for UnusualNames"),
    }
}

#[test]
fn test_const_compatibility_with_enums() {
    #[derive(TypeInfo)]
    #[allow(dead_code)]
    enum ConstTestEnum {
        Variant1,
        Variant2,
    }

    // This should compile at const time
    const _: &Type<'static> = ConstTestEnum::TYPE_INFO;
}
