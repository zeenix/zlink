#![deny(
    missing_debug_implementations,
    nonstandard_style,
    rust_2018_idioms,
    missing_docs
)]
#![warn(unreachable_pub)]
#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;

mod type_info;

/// Derives `TypeInfo` for structs, generating appropriate `Type::Struct` representation.
///
/// This macro only supports structs with named fields (and unit structs). It will generate a
/// `TypeInfo` implementation that creates a `Type::Struct` containing all the fields with their
/// names and types. Tuple structs are not supported as Varlink does not support unnamed fields.
///
/// # Limitations
///
/// The following types are **not** supported by this macro:
///
/// - **Tuple structs**: Varlink does not support unnamed fields
/// - **Enums**: Use the `ReplyError` derive macro for error enums instead
/// - **Unions**: Not supported by Varlink
///
/// ```rust,compile_fail
/// # use zlink::idl::TypeInfo;
/// #[derive(TypeInfo)]  // This will fail to compile
/// struct Point(f32, f32, f32);
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
///     Type::Struct(fields) => {
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
///     Type::Struct(fields) => {
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
///     Type::Struct(fields) => {
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
#[proc_macro_derive(TypeInfo)]
pub fn derive_type_info(input: TokenStream) -> TokenStream {
    type_info::derive_type_info(input)
}
