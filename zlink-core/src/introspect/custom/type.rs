//! Custom type introspection trait.

use crate::idl::custom::{self as custom};

/// Custom type introspection.
///
/// This trait is similar to [`crate::introspect::Type`] but provides custom type definitions
/// that include the type name and are suitable for IDL generation. While
/// [`crate::introspect::Type`] returns inline type information, this trait returns named custom
/// type definitions.
///
/// # Examples
///
/// ```rust
/// use zlink_core::introspect::custom::Type;
/// use zlink_core::idl::{self, Field, custom::{self as custom, Object}};
///
/// struct Point {
///     x: f64,
///     y: f64,
/// }
///
/// impl Type for Point {
///     const TYPE: &'static custom::Type<'static> = &{
///         static FIELD_X: Field<'static> = Field::new("x", &zlink_core::idl::Type::Float);
///         static FIELD_Y: Field<'static> = Field::new("y", &zlink_core::idl::Type::Float);
///         static FIELDS: &[&Field<'static>] = &[&FIELD_X, &FIELD_Y];
///
///         custom::Type::Object(Object::new("Point", FIELDS))
///     };
/// }
/// ```
pub trait Type {
    /// The custom type information including the type name.
    const TYPE: &'static custom::Type<'static>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        idl::{
            self,
            custom::{Enum, Object},
            Field,
        },
        introspect,
    };

    // Test implementation for a custom struct type
    struct TestPoint;

    impl Type for TestPoint {
        const TYPE: &'static custom::Type<'static> = &{
            static FIELD_X: Field<'static> = Field::new("x", <f64 as introspect::Type>::TYPE);
            static FIELD_Y: Field<'static> = Field::new("y", <f64 as introspect::Type>::TYPE);
            static FIELDS: &[&Field<'static>] = &[&FIELD_X, &FIELD_Y];

            custom::Type::Object(Object::new("Point", FIELDS))
        };
    }

    // Test implementation for a custom enum type
    struct TestStatus;

    impl Type for TestStatus {
        const TYPE: &'static custom::Type<'static> = &{
            static VARIANT_ACTIVE: &str = "Active";
            static VARIANT_INACTIVE: &str = "Inactive";
            static VARIANT_PENDING: &str = "Pending";
            static VARIANTS: &[&'static &'static str] =
                &[&VARIANT_ACTIVE, &VARIANT_INACTIVE, &VARIANT_PENDING];

            custom::Type::Enum(Enum::new("Status", VARIANTS))
        };
    }

    #[test]
    fn custom_struct_type() {
        match TestPoint::TYPE {
            custom::Type::Object(obj) => {
                assert_eq!(obj.name(), "Point");

                let fields: mayheap::Vec<_, 8> = obj.fields().collect();
                assert_eq!(fields.len(), 2);

                assert_eq!(fields[0].name(), "x");
                assert_eq!(fields[0].ty(), &idl::Type::Float);

                assert_eq!(fields[1].name(), "y");
                assert_eq!(fields[1].ty(), &idl::Type::Float);
            }
            _ => panic!("Expected custom object type"),
        }
    }

    #[test]
    fn custom_enum_type() {
        match TestStatus::TYPE {
            custom::Type::Enum(enm) => {
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
        // Verify that TYPE can be used in const contexts
        const _POINT_TYPE: &custom::Type<'static> = TestPoint::TYPE;
        const _STATUS_TYPE: &custom::Type<'static> = TestStatus::TYPE;
    }

    #[test]
    fn type_name_access() {
        assert_eq!(TestPoint::TYPE.name(), "Point");
        assert_eq!(TestStatus::TYPE.name(), "Status");
    }

    #[test]
    fn type_variant_checking() {
        assert!(TestPoint::TYPE.is_object());
        assert!(!TestPoint::TYPE.is_enum());

        assert!(!TestStatus::TYPE.is_object());
        assert!(TestStatus::TYPE.is_enum());
    }
}
