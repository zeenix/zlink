//! Custom type introspection trait.

use super::Type;

/// Custom type introspection.
///
/// This trait is similar to [`crate::idl::TypeInfo`] but provides custom type definitions
/// that include the type name and are suitable for IDL generation. While [`crate::idl::TypeInfo`]
/// returns inline type information, this trait returns named custom type definitions.
///
/// # Examples
///
/// ```rust
/// use zlink_core::idl::custom::{TypeInfo, Type, Object, Enum};
/// use zlink_core::idl::Field;
///
/// struct Point {
///     x: f64,
///     y: f64,
/// }
///
/// impl TypeInfo for Point {
///     const TYPE_INFO: &'static Type<'static> = &{
///         static FIELD_X: Field<'static> = Field::new("x", &zlink_core::idl::Type::Float);
///         static FIELD_Y: Field<'static> = Field::new("y", &zlink_core::idl::Type::Float);
///         static FIELDS: &[&Field<'static>] = &[&FIELD_X, &FIELD_Y];
///
///         Type::Object(Object::new("Point", FIELDS))
///     };
/// }
/// ```
pub trait TypeInfo {
    /// The custom type information including the type name.
    const TYPE_INFO: &'static Type<'static>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::idl::{
        custom::{Enum, Object},
        Field, Type as VarlinkType, TypeInfo as VarlinkTypeInfo,
    };

    // Test implementation for a custom struct type
    struct TestPoint;

    impl TypeInfo for TestPoint {
        const TYPE_INFO: &'static Type<'static> = &{
            static FIELD_X: Field<'static> = Field::new("x", <f64 as VarlinkTypeInfo>::TYPE_INFO);
            static FIELD_Y: Field<'static> = Field::new("y", <f64 as VarlinkTypeInfo>::TYPE_INFO);
            static FIELDS: &[&Field<'static>] = &[&FIELD_X, &FIELD_Y];

            Type::Object(Object::new("Point", FIELDS))
        };
    }

    // Test implementation for a custom enum type
    struct TestStatus;

    impl TypeInfo for TestStatus {
        const TYPE_INFO: &'static Type<'static> = &{
            static VARIANT_ACTIVE: &str = "Active";
            static VARIANT_INACTIVE: &str = "Inactive";
            static VARIANT_PENDING: &str = "Pending";
            static VARIANTS: &[&'static &'static str] =
                &[&VARIANT_ACTIVE, &VARIANT_INACTIVE, &VARIANT_PENDING];

            Type::Enum(Enum::new("Status", VARIANTS))
        };
    }

    #[test]
    fn custom_struct_type_info() {
        match TestPoint::TYPE_INFO {
            Type::Object(obj) => {
                assert_eq!(obj.name(), "Point");

                let fields: mayheap::Vec<_, 8> = obj.fields().collect();
                assert_eq!(fields.len(), 2);

                assert_eq!(fields[0].name(), "x");
                assert_eq!(fields[0].ty(), &VarlinkType::Float);

                assert_eq!(fields[1].name(), "y");
                assert_eq!(fields[1].ty(), &VarlinkType::Float);
            }
            _ => panic!("Expected custom object type"),
        }
    }

    #[test]
    fn custom_enum_type_info() {
        match TestStatus::TYPE_INFO {
            Type::Enum(enm) => {
                assert_eq!(enm.name(), "Status");

                let variants: mayheap::Vec<_, 8> = enm.variants().collect();
                assert_eq!(variants.len(), 3);

                assert_eq!(*variants[0], "Active");
                assert_eq!(*variants[1], "Inactive");
                assert_eq!(*variants[2], "Pending");
            }
            _ => panic!("Expected custom enum type"),
        }
    }

    #[test]
    fn const_compatibility() {
        // Verify that TYPE_INFO can be used in const contexts
        const _POINT_TYPE: &Type<'static> = TestPoint::TYPE_INFO;
        const _STATUS_TYPE: &Type<'static> = TestStatus::TYPE_INFO;
    }

    #[test]
    fn type_name_access() {
        assert_eq!(TestPoint::TYPE_INFO.name(), "Point");
        assert_eq!(TestStatus::TYPE_INFO.name(), "Status");
    }

    #[test]
    fn type_variant_checking() {
        assert!(TestPoint::TYPE_INFO.is_object());
        assert!(!TestPoint::TYPE_INFO.is_enum());

        assert!(!TestStatus::TYPE_INFO.is_object());
        assert!(TestStatus::TYPE_INFO.is_enum());
    }
}
