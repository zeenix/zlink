use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DataEnum, DeriveInput, Error, Fields, FieldsNamed};

/// Main entry point for the ReplyError derive macro that generates serde implementations.
///
/// This macro:
/// 1. Generates manual `serde::Serialize` and `serde::Deserialize` implementations
/// 2. Uses allocation-free deserialization for both std and no-std environments
/// 3. Requires "error" field to appear before "parameters" field for efficient parsing
/// 4. Requires `#[zlink(interface = "...")]` to automatically generate qualified error names
/// 5. Handles unit variants with or without empty parameters (serde issue #2045)
pub(crate) fn derive_reply_error(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as DeriveInput);

    let result = derive_reply_error_impl(&ast);

    match result {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn derive_reply_error_impl(input: &DeriveInput) -> Result<TokenStream2, Error> {
    let name = &input.ident;
    let generics = &input.generics;

    // Parse the interface from zlink attributes (mandatory)
    let interface = parse_interface_from_attrs(&input.attrs)?;

    match &input.data {
        Data::Enum(data_enum) => {
            // Validate that enum variants are supported
            validate_enum_variants(data_enum)?;

            // Generate manual Serialize and Deserialize implementations
            let serialize_impl = generate_serialize_impl(name, data_enum, generics, &interface)?;
            let deserialize_impl =
                generate_deserialize_impl(name, data_enum, generics, &interface)?;

            Ok(quote! {
                #serialize_impl
                #deserialize_impl
            })
        }
        Data::Struct(_) => Err(Error::new_spanned(
            input,
            "ReplyError derive macro only supports enums, not structs",
        )),
        Data::Union(_) => Err(Error::new_spanned(
            input,
            "ReplyError derive macro only supports enums, not unions",
        )),
    }
}

/// Parse interface attribute from #[zlink(interface = "...")].
fn parse_interface_from_attrs(attrs: &[syn::Attribute]) -> Result<String, Error> {
    for attr in attrs {
        if attr.path().is_ident("zlink") {
            let mut interface_result = None;
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("interface") {
                    let value = meta.value()?;
                    let lit_str: syn::LitStr = value.parse()?;
                    interface_result = Some(lit_str.value());
                } else {
                    // Skip unknown attributes by consuming their values
                    let _ = meta.value()?;
                    let _: syn::Expr = meta.input.parse()?;
                }
                Ok(())
            })?;

            if let Some(interface) = interface_result {
                return Ok(interface);
            }
        }
    }
    Err(Error::new(
        proc_macro2::Span::call_site(),
        "ReplyError macro requires #[zlink(interface = \"...\")]  attribute",
    ))
}

/// Validate that enum variants are supported by the ReplyError derive macro.
fn validate_enum_variants(data_enum: &DataEnum) -> Result<(), Error> {
    for variant in &data_enum.variants {
        match &variant.fields {
            Fields::Unit => {
                // Unit variants are fine
            }
            Fields::Named(_) => {
                // Named field variants are fine
            }
            Fields::Unnamed(_) => {
                return Err(Error::new_spanned(
                    variant,
                    "ReplyError derive macro does not support tuple variants",
                ));
            }
        }
    }
    Ok(())
}

fn generate_serialize_impl(
    name: &syn::Ident,
    data_enum: &DataEnum,
    generics: &syn::Generics,
    interface: &str,
) -> Result<TokenStream2, Error> {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let has_lifetimes = !generics.lifetimes().collect::<Vec<_>>().is_empty();

    // Generate match arms for each variant (empty for empty enums)
    let variant_arms = data_enum
        .variants
        .iter()
        .map(|variant| generate_serialize_variant_arm(variant, interface, has_lifetimes))
        .collect::<Result<Vec<_>, _>>()?;

    // For empty enums, we need to dereference self to match the uninhabited type
    let match_expr = if data_enum.variants.is_empty() {
        quote! { *self }
    } else {
        quote! { self }
    };

    Ok(quote! {
        impl #impl_generics serde::Serialize for #name #ty_generics #where_clause {
            fn serialize<S>(&self, #[allow(unused)] serializer: S) -> core::result::Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                match #match_expr {
                    #(#variant_arms)*
                }
            }
        }
    })
}

fn generate_serialize_variant_arm(
    variant: &syn::Variant,
    interface: &str,
    has_lifetimes: bool,
) -> Result<TokenStream2, Error> {
    let variant_name = &variant.ident;
    let qualified_name = format!("{interface}.{variant_name}");

    match &variant.fields {
        Fields::Unit => {
            // Unit variant - serialize as tagged enum with just error field
            Ok(quote! {
                Self::#variant_name => {
                    use serde::ser::SerializeMap;
                    let mut map = serializer.serialize_map(Some(1))?;
                    map.serialize_entry("error", #qualified_name)?;
                    map.end()
                }
            })
        }
        Fields::Named(fields) => {
            // Named fields - serialize as tagged enum with parameters
            let field_info = FieldInfo::extract(fields);
            let field_count = field_info.names.len();
            let field_names = &field_info.names;
            let field_types = &field_info.types;
            let field_name_strs = &field_info.name_strings;

            // Convert field types to use synthetic lifetime for ParametersSerializer
            // only if enum has lifetimes
            let serializer_field_types: Vec<syn::Type> = if has_lifetimes {
                field_types
                    .iter()
                    .map(|ty| convert_to_synthetic_lifetime(ty, "'__param"))
                    .collect()
            } else {
                field_types.iter().map(|&ty| ty.clone()).collect()
            };

            Ok(quote! {
                Self::#variant_name { #(#field_names,)* } => {
                    use serde::ser::SerializeMap;

                    let mut map = serializer.serialize_map(Some(2))?;
                    map.serialize_entry("error", #qualified_name)?;

                    // Create a nested "parameters" object
                    map.serialize_entry("parameters", &{
                        use serde::ser::SerializeMap;
                        struct ParametersSerializer<'__param> {
                            #(#field_names: &'__param #serializer_field_types,)*
                        }

                        impl<'__param> serde::Serialize for ParametersSerializer<'__param> {
                            fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
                            where
                                S: serde::Serializer,
                            {
                                let mut map = serializer.serialize_map(Some(#field_count))?;
                                #(
                                    map.serialize_entry(#field_name_strs, self.#field_names)?;
                                )*
                                map.end()
                            }
                        }

                        ParametersSerializer {
                            #(#field_names,)*
                        }
                    })?;

                    map.end()
                }
            })
        }
        Fields::Unnamed(_) => Err(Error::new_spanned(
            variant,
            "ReplyError derive macro does not support tuple variants",
        )),
    }
}

fn generate_deserialize_impl(
    name: &syn::Ident,
    data_enum: &DataEnum,
    generics: &syn::Generics,
    interface: &str,
) -> Result<TokenStream2, Error> {
    let has_lifetimes = !generics.lifetimes().collect::<Vec<_>>().is_empty();

    // Create impl generics with proper lifetime bounds
    let mut impl_generics = generics.clone();
    impl_generics.params.insert(0, syn::parse_quote!('de));

    // Add lifetime bounds for serde pattern: 'de must outlive all enum lifetimes
    if has_lifetimes {
        let enum_lifetimes: Vec<_> = generics.lifetimes().collect();
        for lifetime in &enum_lifetimes {
            let lifetime_ident = &lifetime.lifetime;
            impl_generics
                .make_where_clause()
                .predicates
                .push(syn::parse_quote!('de: #lifetime_ident));
        }
    }

    let (impl_generics_tokens, _, where_clause) = impl_generics.split_for_impl();
    let (_, ty_generics, _) = generics.split_for_impl();

    // Handle empty enums specially
    if data_enum.variants.is_empty() {
        return Ok(quote! {
            impl #impl_generics_tokens serde::Deserialize<'de> for #name #ty_generics #where_clause {
                fn deserialize<D>(_deserializer: D) -> core::result::Result<Self, D::Error>
                where
                    D: serde::Deserializer<'de>,
                {
                    use serde::de;
                    Err(de::Error::custom("cannot deserialize empty enum"))
                }
            }
        });
    }

    // For visitor Value type, convert enum lifetimes to 'de
    let visitor_ty_generics = generate_visitor_ty_generics(generics, has_lifetimes);

    // Generate match arms for each variant
    let variant_arms = generate_variant_match_arms(name, data_enum, interface, has_lifetimes)?;

    // Generate the variant names for error reporting
    let variant_names: Vec<String> = data_enum
        .variants
        .iter()
        .map(|v| format!("{}.{}", interface, v.ident))
        .collect();

    // Generate visitor struct name
    let visitor_name = quote::format_ident!("{}Visitor", name);

    Ok(quote! {
        impl #impl_generics_tokens serde::Deserialize<'de> for #name #ty_generics #where_clause {
            fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct #visitor_name;

                impl<'de> serde::de::Visitor<'de> for #visitor_name {
                    type Value = #name #visitor_ty_generics;

                    fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                        formatter.write_str(concat!("a ", stringify!(#name), " error"))
                    }

                    fn visit_map<M>(self, mut map: M) -> core::result::Result<Self::Value, M::Error>
                    where
                        M: serde::de::MapAccess<'de>,
                    {
                        use serde::de;

                        // Allocation-free approach: require "error" field to be first.
                        let key = map.next_key::<&str>()?;
                        if key != Some("error") {
                            return Err(de::Error::custom("expected 'error' field first"));
                        }
                        let error_type: &str = map.next_value()?;

                        // Match on the error type and deserialize parameters if present
                        match error_type {
                            #variant_arms
                            _ => Err(de::Error::unknown_variant(
                                error_type,
                                &[#(#variant_names),*],
                            ))
                        }
                    }
                }

                deserializer.deserialize_map(#visitor_name)
            }
        }
    })
}

fn generate_visitor_ty_generics(generics: &syn::Generics, has_lifetimes: bool) -> TokenStream2 {
    if has_lifetimes {
        // Generate token stream with lifetimes converted to 'de
        let converted_params: Vec<_> = generics
            .params
            .iter()
            .map(|param| match param {
                syn::GenericParam::Lifetime(_) => quote! { 'de },
                syn::GenericParam::Type(type_param) => {
                    let ident = &type_param.ident;
                    quote! { #ident }
                }
                syn::GenericParam::Const(const_param) => {
                    let ident = &const_param.ident;
                    quote! { #ident }
                }
            })
            .collect();

        if converted_params.is_empty() {
            quote! {}
        } else {
            quote! { <#(#converted_params),*> }
        }
    } else {
        let (_, orig_ty_generics, _) = generics.split_for_impl();
        quote! { #orig_ty_generics }
    }
}

/// Generate variant match arms for deserialization using allocation-free approach.
fn generate_variant_match_arms(
    enum_name: &syn::Ident,
    data_enum: &DataEnum,
    interface: &str,
    has_lifetimes: bool,
) -> Result<TokenStream2, Error> {
    let mut arms = Vec::new();

    for variant in &data_enum.variants {
        let variant_name = &variant.ident;
        let qualified_name = format!("{interface}.{variant_name}");

        match &variant.fields {
            Fields::Unit => {
                // Unit variant - skip remaining fields, including optional "parameters" field
                let unit_arm = quote! {
                    #qualified_name => {
                        // Skip remaining fields, including optional "parameters" field
                        while map.next_key::<&str>()?.is_some() {
                            let _: de::IgnoredAny = map.next_value()?;
                        }
                        Ok(#enum_name::#variant_name)
                    }
                };
                arms.push(unit_arm);
            }
            Fields::Named(fields) => {
                // Named fields - deserialize from parameters object
                let field_info = FieldInfo::extract(fields);
                let visitor_code = generate_parameters_visitor(&field_info, has_lifetimes);
                let field_names = &field_info.names;

                let named_arm = quote! {
                    #qualified_name => {
                        // We need the parameters field next
                        let key = map.next_key::<&str>()?;
                        if key != Some("parameters") {
                            // No parameters field, which means this is a unit variant
                            // We should not reach here for named variants
                            // since they should have parameters
                            return Err(de::Error::custom("named field variant requires parameters field"));
                        }

                        // Use a custom visitor to deserialize parameters directly
                        #visitor_code

                        let (#(#field_names,)*) = map.next_value_seed(ParametersVisitor)?;

                        // Skip any remaining fields
                        while map.next_key::<&str>()?.is_some() {
                            let _: de::IgnoredAny = map.next_value()?;
                        }

                        Ok(#enum_name::#variant_name { #(#field_names,)* })
                    }
                };
                arms.push(named_arm);
            }
            Fields::Unnamed(_) => {
                return Err(Error::new_spanned(
                    variant,
                    "ReplyError derive macro does not support tuple variants",
                ));
            }
        }
    }

    Ok(quote! { #(#arms)* })
}

/// Generate visitor pattern code for deserializing named field parameters.
fn generate_parameters_visitor(field_info: &FieldInfo<'_>, has_lifetimes: bool) -> TokenStream2 {
    let field_names = &field_info.names;
    let field_types = &field_info.types;
    let field_name_strs = &field_info.name_strings;

    // Convert field types to use 'de lifetime for visitor
    let visitor_field_types: Vec<syn::Type> = if has_lifetimes {
        field_types
            .iter()
            .map(|ty| convert_to_de_lifetime(ty))
            .collect()
    } else {
        field_types.iter().map(|&ty| ty.clone()).collect()
    };

    // Zip field names with their visitor types for proper repetition
    let field_declarations = field_names
        .iter()
        .zip(&visitor_field_types)
        .map(|(name, ty)| {
            quote! { let mut #name: Option<#ty> = None; }
        });

    let field_assignments =
        field_name_strs
            .iter()
            .zip(field_names.iter())
            .map(|(name_str, name)| {
                quote! {
                    #name_str => {
                        if #name.is_some() {
                            return Err(de::Error::duplicate_field(#name_str));
                        }
                        #name = Some(map.next_value()?);
                    }
                }
            });

    let field_extractions =
        field_names
            .iter()
            .zip(field_name_strs.iter())
            .map(|(name, name_str)| {
                quote! { #name.ok_or_else(|| de::Error::missing_field(#name_str))? }
            });

    quote! {
        struct ParametersVisitor;

        impl<'de> de::DeserializeSeed<'de> for ParametersVisitor {
            type Value = (#(#visitor_field_types,)*);

            fn deserialize<D>(self, deserializer: D) -> core::result::Result<Self::Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> de::Visitor<'de> for FieldVisitor {
                    type Value = (#(#visitor_field_types,)*);

                    fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                        formatter.write_str("parameters object")
                    }

                    fn visit_map<M>(self, mut map: M) -> core::result::Result<Self::Value, M::Error>
                    where
                        M: de::MapAccess<'de>,
                    {
                        #(#field_declarations)*

                        while let Some(key) = map.next_key::<&str>()? {
                            match key {
                                #(#field_assignments)*
                                _ => {
                                    let _: de::IgnoredAny = map.next_value()?;
                                }
                            }
                        }

                        Ok((
                            #(#field_extractions,)*
                        ))
                    }
                }

                deserializer.deserialize_map(FieldVisitor)
            }
        }
    }
}

/// Convert enum lifetimes to a specific synthetic lifetime.
fn convert_to_synthetic_lifetime(ty: &syn::Type, target_lifetime: &str) -> syn::Type {
    let target_lt: syn::Lifetime = syn::parse_str(target_lifetime).unwrap();

    match ty {
        syn::Type::Reference(type_ref) => {
            let elem = convert_to_synthetic_lifetime(&type_ref.elem, target_lifetime);
            syn::Type::Reference(syn::TypeReference {
                and_token: type_ref.and_token,
                lifetime: Some(target_lt),
                mutability: type_ref.mutability,
                elem: Box::new(elem),
            })
        }
        syn::Type::Path(type_path) => {
            let mut new_path = type_path.clone();
            for segment in &mut new_path.path.segments {
                let syn::PathArguments::AngleBracketed(args) = &mut segment.arguments else {
                    continue;
                };
                let mut new_args = args.clone();
                new_args.args = args
                    .args
                    .iter()
                    .map(|arg| match arg {
                        syn::GenericArgument::Type(inner_ty) => syn::GenericArgument::Type(
                            convert_to_synthetic_lifetime(inner_ty, target_lifetime),
                        ),
                        syn::GenericArgument::Lifetime(_) => {
                            syn::GenericArgument::Lifetime(target_lt.clone())
                        }
                        _ => arg.clone(),
                    })
                    .collect();
                segment.arguments = syn::PathArguments::AngleBracketed(new_args);
            }
            syn::Type::Path(new_path)
        }
        syn::Type::Slice(type_slice) => {
            let elem = convert_to_synthetic_lifetime(&type_slice.elem, target_lifetime);
            syn::Type::Slice(syn::TypeSlice {
                bracket_token: type_slice.bracket_token,
                elem: Box::new(elem),
            })
        }
        syn::Type::Array(type_array) => {
            let elem = convert_to_synthetic_lifetime(&type_array.elem, target_lifetime);
            syn::Type::Array(syn::TypeArray {
                bracket_token: type_array.bracket_token,
                elem: Box::new(elem),
                semi_token: type_array.semi_token,
                len: type_array.len.clone(),
            })
        }
        syn::Type::Tuple(type_tuple) => {
            let elems = type_tuple
                .elems
                .iter()
                .map(|elem| convert_to_synthetic_lifetime(elem, target_lifetime))
                .collect();
            syn::Type::Tuple(syn::TypeTuple {
                paren_token: type_tuple.paren_token,
                elems,
            })
        }
        _ => ty.clone(),
    }
}

/// Convert enum lifetimes to 'de lifetime for visitor types.
fn convert_to_de_lifetime(ty: &syn::Type) -> syn::Type {
    convert_to_synthetic_lifetime(ty, "'de")
}

/// Field information extracted from named fields for reuse across
/// serialization/deserialization.
struct FieldInfo<'a> {
    names: Vec<&'a syn::Ident>,
    types: Vec<&'a syn::Type>,
    name_strings: Vec<String>,
}

impl<'a> FieldInfo<'a> {
    /// Extract field information from named fields to avoid duplication.
    fn extract(fields: &'a FieldsNamed) -> Self {
        let field_data: Vec<_> = fields
            .named
            .iter()
            .filter_map(|f| f.ident.as_ref().map(|name| (name, &f.ty)))
            .collect();

        let names: Vec<_> = field_data.iter().map(|(name, _)| *name).collect();
        let types: Vec<_> = field_data.iter().map(|(_, ty)| *ty).collect();
        let name_strings: Vec<String> = names.iter().map(|f| f.to_string()).collect();

        Self {
            names,
            types,
            name_strings,
        }
    }
}
