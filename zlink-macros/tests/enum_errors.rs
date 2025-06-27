//! Tests for Type derive macro error handling and edge cases.

use zlink::{idl, introspect::Type};

#[test]
fn test_unit_enum_works() {
    #[derive(Type)]
    #[allow(dead_code)]
    enum SimpleEnum {
        First,
        Second,
        Third,
    }

    match SimpleEnum::TYPE {
        idl::Type::Enum(variants) => {
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
    #[derive(Type)]
    #[allow(dead_code)]
    enum SingleVariant {
        Only,
    }

    match SingleVariant::TYPE {
        idl::Type::Enum(variants) => {
            let variant_vec: Vec<_> = variants.iter().collect();
            assert_eq!(variant_vec.len(), 1);
            assert_eq!(*variant_vec[0], "Only");
        }
        _ => panic!("Expected enum type for SingleVariant"),
    }
}

// The following would cause compile-time errors if uncommented:
//
// #[derive(Type)]
// enum TupleVariantEnum {
//     Active(String),  // Error: Type derive macro only supports unit enum variants, not tuple
// variants     Inactive,
// }
//
// #[derive(Type)]
// enum StructVariantEnum {
//     Active { status: String },  // Error: Type derive macro only supports unit enum variants,
// not struct variants     Inactive,
// }
//
// #[derive(Type)]
// enum MixedEnum {
//     Unit,
//     Tuple(i32),  // Error: Type derive macro only supports unit enum variants, not tuple
// variants     Struct { field: String },  // Error: Type derive macro only supports unit enum
// variants, not struct variants }

#[test]
fn test_enum_with_many_variants() {
    #[derive(Type)]
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

    match ManyVariants::TYPE {
        idl::Type::Enum(variants) => {
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
    #[derive(Type)]
    #[allow(dead_code)]
    #[allow(non_camel_case_types)]
    enum UnusualNames {
        CamelCase,
        snake_case,
        UPPER_CASE,
        Mixed_Case_123,
    }

    match UnusualNames::TYPE {
        idl::Type::Enum(variants) => {
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
    #[derive(Type)]
    #[allow(dead_code)]
    enum ConstTestEnum {
        Variant1,
        Variant2,
    }

    // This should compile at const time
    const _: &idl::Type<'static> = ConstTestEnum::TYPE;
}
