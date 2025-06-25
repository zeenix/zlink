#![deny(
    missing_debug_implementations,
    nonstandard_style,
    rust_2018_idioms,
    missing_docs
)]
#![warn(unreachable_pub)]
#![doc = include_str!("../README.md")]

#[cfg(feature = "idl")]
mod type_info;

#[cfg(feature = "idl")]
mod custom_type_info;

/// Derives `TypeInfo` for structs and enums, generating appropriate `Type::Object` or `Type::Enum`
/// representation.
///
/// ## Structs
///
/// For structs, this macro supports named fields and unit structs. It will generate a
/// `TypeInfo` implementation that creates a `Type::Object` containing all the fields with their
/// names and types. Tuple structs are not supported as Varlink does not support unnamed fields.
///
/// ## Enums
///
/// For enums, this macro only supports unit variants (variants without associated data). It will
/// generate a `TypeInfo` implementation that creates a `Type::Enum` containing all the variant
/// names.
///
/// # Limitations
///
/// The following types are **not** supported by this macro:
///
/// - **Tuple structs**: Varlink does not support unnamed fields
/// - **Enums with data**: Only unit enums (variants without associated data) are supported
/// - **Unions**: Not supported by Varlink
///
/// ```rust,compile_fail
/// # use zlink::idl::TypeInfo;
/// #[derive(TypeInfo)]  // This will fail to compile
/// struct Point(f32, f32, f32);
/// ```
///
/// ```rust,compile_fail
/// # use zlink::idl::TypeInfo;
/// #[derive(TypeInfo)]  // This will fail to compile
/// enum Status {
///     Active(String),  // Variants with data are not supported
///     Inactive,
/// }
/// ```
///
/// # Examples
///
/// ## Named Structs
///
/// ```rust
/// use zlink::idl::{TypeInfo, Type};
///
/// #[derive(TypeInfo)]
/// struct Person {
///     name: String,
///     age: i32,
///     active: bool,
/// }
///
/// // Access the generated type information
/// match Person::TYPE_INFO {
///     Type::Object(fields) => {
///         let field_vec: Vec<_> = fields.iter().collect();
///         assert_eq!(field_vec.len(), 3);
///
///         assert_eq!(field_vec[0].name(), "name");
///         assert_eq!(field_vec[0].ty(), &Type::String);
///
///         assert_eq!(field_vec[1].name(), "age");
///         assert_eq!(field_vec[1].ty(), &Type::Int);
///
///         assert_eq!(field_vec[2].name(), "active");
///         assert_eq!(field_vec[2].ty(), &Type::Bool);
///     }
///     _ => panic!("Expected struct type"),
/// }
/// ```
///
/// ## Unit Structs
///
/// ```rust
/// # use zlink::idl::{TypeInfo, Type};
/// #[derive(TypeInfo)]
/// struct Unit;
///
/// // Unit structs generate empty field lists
/// match Unit::TYPE_INFO {
///     Type::Object(fields) => {
///         assert_eq!(fields.len(), 0);
///     }
///     _ => panic!("Expected struct type"),
/// }
/// ```
///
/// ## Complex Types
///
/// ```rust
/// # use zlink::idl::{TypeInfo, Type};
/// #[derive(TypeInfo)]
/// struct Complex {
///     id: u64,
///     description: Option<String>,
///     tags: Vec<String>,
/// }
///
/// // The macro handles nested types like Option<T> and Vec<T>
/// match Complex::TYPE_INFO {
///     Type::Object(fields) => {
///         let field_vec: Vec<_> = fields.iter().collect();
///
///         // Optional field becomes Type::Optional
///         match field_vec[1].ty() {
///             Type::Optional(inner) => assert_eq!(inner.inner(), &Type::String),
///             _ => panic!("Expected optional type"),
///         }
///
///         // Vec field becomes Type::Array
///         match field_vec[2].ty() {
///             Type::Array(inner) => assert_eq!(inner.inner(), &Type::String),
///             _ => panic!("Expected array type"),
///         }
///     }
///     _ => panic!("Expected struct type"),
/// }
/// ```
///
/// ## Unit Enums
///
/// ```rust
/// # use zlink::idl::{TypeInfo, Type};
/// #[derive(TypeInfo)]
/// enum Status {
///     Active,
///     Inactive,
///     Pending,
/// }
///
/// // Unit enums generate variant lists
/// match Status::TYPE_INFO {
///     Type::Enum(variants) => {
///         let variant_vec: Vec<_> = variants.iter().collect();
///         assert_eq!(variant_vec.len(), 3);
///         assert_eq!(*variant_vec[0], "Active");
///         assert_eq!(*variant_vec[1], "Inactive");
///         assert_eq!(*variant_vec[2], "Pending");
///     }
///     _ => panic!("Expected enum type"),
/// }
/// ```
///
/// This macro is only available when the `idl` feature is enabled.
#[proc_macro_derive(TypeInfo)]
#[cfg(feature = "idl")]
pub fn derive_type_info(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    type_info::derive_type_info(input)
}

/// Derives `TypeInfo` for structs and enums, generating named custom type definitions.
///
/// This macro generates implementations of the `TypeInfo` trait, which provides named
/// custom type definitions suitable for IDL generation. Unlike the regular `TypeInfo` derive,
/// this macro includes the type name in the generated type information.
///
/// ## Structs
///
/// For structs, this macro generates a `custom::Type::Object` containing the struct name and
/// all fields with their names and types.
///
/// ## Enums
///
/// For enums, this macro only supports unit variants and generates a `custom::Type::Enum`
/// containing the enum name and all variant names.
///
/// # Examples
///
/// ## Named Structs
///
/// ```rust
/// use zlink::idl::custom::{TypeInfo, Type};
///
/// #[derive(TypeInfo)]
/// struct Point {
///     x: f64,
///     y: f64,
/// }
///
/// // Access the generated custom type information
/// match Point::TYPE_INFO {
///     Type::Object(obj) => {
///         assert_eq!(obj.name(), "Point");
///         let fields: Vec<_> = obj.fields().collect();
///         assert_eq!(fields.len(), 2);
///         assert_eq!(fields[0].name(), "x");
///         assert_eq!(fields[1].name(), "y");
///     }
///     _ => panic!("Expected custom object type"),
/// }
/// ```
///
/// ## Unit Enums
///
/// ```rust
/// # use zlink::idl::custom::{TypeInfo, Type};
/// #[derive(TypeInfo)]
/// enum Status {
///     Active,
///     Inactive,
///     Pending,
/// }
///
/// // Access the generated custom enum type information
/// match Status::TYPE_INFO {
///     Type::Enum(enm) => {
///         assert_eq!(enm.name(), "Status");
///         let variants: Vec<_> = enm.variants().collect();
///         assert_eq!(variants.len(), 3);
///         assert_eq!(*variants[0], "Active");
///         assert_eq!(*variants[1], "Inactive");
///         assert_eq!(*variants[2], "Pending");
///     }
///     _ => panic!("Expected custom enum type"),
/// }
/// ```
///
/// This macro is only available when the `idl` feature is enabled.
#[proc_macro_derive(CustomTypeInfo)]
#[cfg(feature = "idl")]
pub fn derive_custom_type_info(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    custom_type_info::derive_custom_type_info(input)
}
