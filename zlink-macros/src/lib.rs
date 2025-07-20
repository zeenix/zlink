#![deny(
    missing_debug_implementations,
    nonstandard_style,
    rust_2018_idioms,
    missing_docs
)]
#![warn(unreachable_pub)]
#![doc = include_str!("../README.md")]

#[cfg(feature = "introspection")]
mod utils;

#[cfg(feature = "introspection")]
mod r#type;

#[cfg(feature = "introspection")]
mod custom_type;

#[cfg(feature = "introspection")]
mod reply_error;

#[cfg(feature = "proxy")]
mod proxy;

/// Derives `Type` for structs and enums, generating appropriate `Type::Object` or `Type::Enum`
/// representation.
///
/// **Requires the `introspection` feature to be enabled.**
///
/// ## Structs
///
/// For structs, this macro supports named fields and unit structs. It will generate a
/// `Type` implementation that creates a `Type::Object` containing all the fields with their
/// names and types. Tuple structs are not supported as Varlink does not support unnamed fields.
///
/// ## Enums
///
/// For enums, this macro only supports unit variants (variants without associated data). It will
/// generate a `Type` implementation that creates a `Type::Enum` containing all the variant
/// names.
///
/// # Supported Attributes
///
/// The following attributes can be used to customize the behavior of this derive macro:
///
/// * `#[zlink(crate = "path")]` - Specifies the crate path to use for zlink types. Defaults to
///   `::zlink`.
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
/// # use zlink::introspect::Type;
/// #[derive(Type)]  // This will fail to compile
/// struct Point(f32, f32, f32);
/// ```
///
/// ```rust,compile_fail
/// # use zlink::introspect::Type;
/// #[derive(Type)]  // This will fail to compile
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
/// use zlink::introspect::Type;
/// use zlink::idl;
///
/// #[derive(Type)]
/// struct Person {
///     name: String,
///     age: i32,
///     active: bool,
/// }
///
/// // Access the generated type information
/// match Person::TYPE {
///     idl::Type::Object(fields) => {
///         let field_vec: Vec<_> = fields.iter().collect();
///         assert_eq!(field_vec.len(), 3);
///
///         assert_eq!(field_vec[0].name(), "name");
///         assert_eq!(field_vec[0].ty(), &idl::Type::String);
///
///         assert_eq!(field_vec[1].name(), "age");
///         assert_eq!(field_vec[1].ty(), &idl::Type::Int);
///
///         assert_eq!(field_vec[2].name(), "active");
///         assert_eq!(field_vec[2].ty(), &idl::Type::Bool);
///     }
///     _ => panic!("Expected struct type"),
/// }
/// ```
///
/// ## Unit Structs
///
/// ```rust
/// # use zlink::introspect::Type;
/// # use zlink::idl;
/// #[derive(Type)]
/// struct Unit;
///
/// // Unit structs generate empty field lists
/// match Unit::TYPE {
///     idl::Type::Object(fields) => {
///         assert_eq!(fields.len(), 0);
///     }
///     _ => panic!("Expected struct type"),
/// }
/// ```
///
/// ## Complex Types
///
/// ```rust
/// # use zlink::introspect::Type;
/// # use zlink::idl;
/// #[derive(Type)]
/// struct Complex {
///     id: u64,
///     description: Option<String>,
///     tags: Vec<String>,
/// }
///
/// // The macro handles nested types like Option<T> and Vec<T>
/// match Complex::TYPE {
///     idl::Type::Object(fields) => {
///         let field_vec: Vec<_> = fields.iter().collect();
///
///         // Optional field becomes Type::Optional
///         match field_vec[1].ty() {
///             idl::Type::Optional(inner) => assert_eq!(inner.inner(), &idl::Type::String),
///             _ => panic!("Expected optional type"),
///         }
///
///         // Vec field becomes Type::Array
///         match field_vec[2].ty() {
///             idl::Type::Array(inner) => assert_eq!(inner.inner(), &idl::Type::String),
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
/// # use zlink::introspect::Type;
/// # use zlink::idl;
/// #[derive(Type)]
/// enum Status {
///     Active,
///     Inactive,
///     Pending,
/// }
///
/// // Unit enums generate variant lists
/// match Status::TYPE {
///     idl::Type::Enum(variants) => {
///         let variant_vec: Vec<_> = variants.iter().collect();
///         assert_eq!(variant_vec.len(), 3);
///         assert_eq!(variant_vec[0].name(), "Active");
///         assert_eq!(variant_vec[1].name(), "Inactive");
///         assert_eq!(variant_vec[2].name(), "Pending");
///     }
///     _ => panic!("Expected enum type"),
/// }
/// ```
#[cfg(feature = "introspection")]
#[proc_macro_derive(Type, attributes(zlink))]
pub fn derive_type(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    r#type::derive_type(input)
}

/// Derives `Type` for structs and enums, generating named custom type definitions.
///
/// **Requires the `introspection` feature to be enabled.**
///
/// This macro generates implementations of the `CustomType` trait, which provides named
/// custom type definitions suitable for IDL generation. It also generates a `Type` implementation
/// and therefore is mutually exclusive to [`Type`] derive macro.
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
/// # Supported Attributes
///
/// The following attributes can be used to customize the behavior of this derive macro:
///
/// * `#[zlink(crate = "path")]` - Specifies the crate path to use for zlink types. Defaults to
///   `::zlink`.
///
/// # Examples
///
/// ## Named Structs
///
/// ```rust
/// use zlink::introspect::{CustomType, Type};
/// use zlink::idl;
///
/// #[derive(CustomType)]
/// struct Point {
///     x: f64,
///     y: f64,
/// }
///
/// // Access the generated custom type information
/// match Point::CUSTOM_TYPE {
///     idl::CustomType::Object(obj) => {
///         assert_eq!(obj.name(), "Point");
///         let fields: Vec<_> = obj.fields().collect();
///         assert_eq!(fields.len(), 2);
///         assert_eq!(fields[0].name(), "x");
///         assert_eq!(fields[1].name(), "y");
///     }
///     _ => panic!("Expected custom object type"),
/// }
///
/// match Point::TYPE {
///     idl::Type::Custom(name) => {
///         assert_eq!(*name, "Point");
///     }
///     _ => panic!("Expected custom type"),
/// }
/// ```
///
/// ## Unit Enums
///
/// ```rust
/// # use zlink::introspect::{CustomType, Type};
/// # use zlink::idl;
/// #[derive(CustomType)]
/// enum Status {
///     Active,
///     Inactive,
///     Pending,
/// }
///
/// // Access the generated custom enum type information
/// match Status::CUSTOM_TYPE {
///     idl::CustomType::Enum(enm) => {
///         assert_eq!(enm.name(), "Status");
///         let variants: Vec<_> = enm.variants().collect();
///         assert_eq!(variants.len(), 3);
///         assert_eq!(variants[0].name(), "Active");
///         assert_eq!(variants[1].name(), "Inactive");
///         assert_eq!(variants[2].name(), "Pending");
///     }
///     _ => panic!("Expected custom enum type"),
/// }
///
/// match Status::TYPE {
///     idl::Type::Custom(name) => {
///         assert_eq!(*name, "Status");
///     }
///     _ => panic!("Expected custom type"),
/// }
/// ```
#[cfg(feature = "introspection")]
#[proc_macro_derive(CustomType, attributes(zlink))]
pub fn derive_custom_type(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    custom_type::derive_custom_type(input)
}

/// Derives `ReplyError` for enums, generating error definitions for Varlink service errors.
///
/// **Requires the `introspection` feature to be enabled.**
///
/// This macro generates implementations of the `ReplyError` trait, which provides a list of
/// error variants that can be returned by a Varlink service method. It supports unit variants,
/// variants with named fields, and single-field tuple variants (where the field type implements
/// `Type` and has a `Type::Object`).
///
/// # Supported Attributes
///
/// The following attributes can be used to customize the behavior of this derive macro:
///
/// * `#[zlink(crate = "path")]` - Specifies the crate path to use for zlink types. Defaults to
///   `::zlink`.
///
/// # Example
///
/// ```rust
/// use zlink::introspect::ReplyError;
///
/// #[derive(ReplyError)]
/// enum ServiceError {
///     // Unit variant - no parameters
///     NotFound,
///
///     // Named field variant - multiple parameters
///     InvalidQuery {
///         message: String,
///         line: u32,
///     },
///
///     // Single tuple variant - uses fields from the wrapped type
///     ValidationFailed(ValidationDetails),
/// }
///
/// // Example struct for tuple variant
/// #[derive(zlink::introspect::Type)]
/// struct ValidationDetails {
///     field_name: String,
///     expected: String,
/// }
///
/// // Access the generated error variants
/// assert_eq!(ServiceError::VARIANTS.len(), 3);
/// assert_eq!(ServiceError::VARIANTS[0].name(), "NotFound");
/// assert!(ServiceError::VARIANTS[0].has_no_fields());
///
/// assert_eq!(ServiceError::VARIANTS[1].name(), "InvalidQuery");
/// assert!(!ServiceError::VARIANTS[1].has_no_fields());
/// ```
#[cfg(feature = "introspection")]
#[proc_macro_derive(ReplyError, attributes(zlink))]
pub fn derive_reply_error(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    reply_error::derive_reply_error(input)
}

/// Creates a client-side proxy for calling Varlink methods on a connection.
///
/// **Requires the `proxy` feature to be enabled.**
///
/// This attribute macro generates an implementation of the provided trait for `Connection<S>`,
/// automatically handling the serialization of method calls and deserialization of responses.
/// Each proxy trait targets a single Varlink interface.
///
/// # Example
///
/// ```rust
/// use zlink::proxy;
/// use serde::{Deserialize, Serialize};
/// use serde_prefix_all::prefix_all;
///
/// #[proxy("org.example.MyService")]
/// trait MyServiceProxy {
///     async fn get_status(&mut self) -> zlink::Result<Result<Status<'_>, MyError<'_>>>;
///     async fn set_value(
///         &mut self,
///         key: &str,
///         value: i32,
///     ) -> zlink::Result<Result<(), MyError<'_>>>;
///     // This will call the `io.systemd.Machine.List` method when `list_machines()` is invoked.
///     #[zlink(rename = "ListMachines")]
///     async fn list_machines(&mut self) -> zlink::Result<Result<Vec<Machine<'_>>, MyError<'_>>>;
/// }
///
/// // The macro generates:
/// // impl<S: Socket> MyServiceProxy for Connection<S> { ... }
///
/// #[derive(Debug, Serialize, Deserialize)]
/// struct Status<'m> {
///     active: bool,
///     message: &'m str,
/// }
///
/// #[derive(Debug, Serialize, Deserialize)]
/// struct Machine<'m> { name: &'m str }
///
/// #[prefix_all("org.example.MyService.")]
/// #[derive(Debug, Serialize, Deserialize)]
/// #[serde(tag = "error", content = "parameters")]
/// enum MyError<'a> {
///     NotFound,
///     InvalidRequest,
///     // Parameters must be named.
///     CodedError { code: u32, message: &'a str },
/// }
/// ```
///
/// # Method Requirements
///
/// Proxy methods must:
/// - Take `&mut self` as the first parameter
/// - Can be either `async fn` or return `impl Future`
/// - Return `zlink::Result<Result<ReplyType, ErrorType>>` (outer Result for connection errors,
///   inner for method errors)
/// - The arguments can be any type that implement `serde::Serialize`
/// - The reply type (`Ok` case of the inner `Result`) must be a type that implements
///   `serde::Deserialize` and deserializes itself from a JSON object. Typically you'd just use a
///   struct that derives `serde::Deserialize`.
/// - The reply error type (`Err` case of the inner `Result`) must be a type `serde::Deserialize`
///   that deserializes itself from a JSON object with two fields:
///   - `error`: a string containing the fully qualified error name
///   - `parameters`: an optional object containing all the fields of the error
///
/// # Method Names
///
/// By default, method names are converted from snake_case to PascalCase for the Varlink call.
/// To specify a different Varlink method name, use the `#[zlink(rename = "...")]` attribute. See
/// `list_machines` in the example above.
#[cfg(feature = "proxy")]
#[proc_macro_attribute]
pub fn proxy(
    attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    proxy::proxy(attr.into(), input.into()).into()
}
