use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DataEnum, DeriveInput, Error, Fields, FieldsNamed, FieldsUnnamed};

use crate::utils;

/// Main entry point for the custom Type derive macro.
pub(crate) fn derive_custom_type(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);

    match derive_custom_type_impl(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn derive_custom_type_impl(input: DeriveInput) -> Result<TokenStream2, Error> {
    let name = &input.ident;
    let name_str = name.to_string();
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let crate_path = utils::parse_crate_path(&input.attrs)?;

    let custom_type = match &input.data {
        Data::Struct(data_struct) => {
            let fields = &data_struct.fields;
            let (field_statics, field_refs) = generate_field_definitions(fields, &crate_path)?;

            quote!({
                #(#field_statics)*

                static FIELD_REFS: &[&#crate_path::idl::Field<'static>] = &[
                    #(#field_refs),*
                ];

                #crate_path::idl::CustomType::Object(
                    #crate_path::idl::CustomObject::new(#name_str, FIELD_REFS, &[])
                )
            })
        }
        Data::Enum(data_enum) => {
            let variant_names = generate_enum_variant_definitions(data_enum)?;

            quote!({
                #crate_path::idl::CustomType::Enum(
                   #crate_path::idl::CustomEnum::new(#name_str, &[
                        #(#variant_names),*
                    ], &[])
                )
            })
        }
        Data::Union(_) => {
            return Err(Error::new_spanned(
                input,
                "Type derive macro only supports structs and enums, not unions",
            ));
        }
    };

    Ok(quote! {
        impl #impl_generics #crate_path::introspect::CustomType for #name #ty_generics #where_clause {
            const CUSTOM_TYPE: &'static #crate_path::idl::CustomType<'static> = &#custom_type;
        }

        impl #impl_generics #crate_path::introspect::Type for #name #ty_generics #where_clause {
            const TYPE: &'static #crate_path::idl::Type<'static> = &#crate_path::idl::Type::Custom(#name_str);
        }
    })
}

fn generate_field_definitions(
    fields: &Fields,
    crate_path: &TokenStream2,
) -> Result<(Vec<TokenStream2>, Vec<TokenStream2>), Error> {
    match fields {
        Fields::Named(FieldsNamed { named, .. }) => {
            let mut field_statics = Vec::new();
            let mut field_refs = Vec::new();

            for field in named {
                let field_name = field
                    .ident
                    .as_ref()
                    .ok_or_else(|| Error::new_spanned(field, "Field must have a name"))?;

                let field_type = utils::remove_lifetimes_from_type(&field.ty);
                let field_name_str = field_name.to_string();
                let static_name =
                    quote::format_ident!("FIELD_{}", field_name.to_string().to_uppercase());

                let field_static = quote! {
                    static #static_name: #crate_path::idl::Field<'static> =
                        #crate_path::idl::Field::new(
                            #field_name_str,
                            <#field_type as #crate_path::introspect::Type>::TYPE,
                            &[]
                        );
                };

                let field_ref = quote! { &#static_name };

                field_statics.push(field_static);
                field_refs.push(field_ref);
            }

            Ok((field_statics, field_refs))
        }
        Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => Err(Error::new_spanned(
            unnamed,
            "Only named fields are supported",
        )),
        Fields::Unit => {
            // Unit structs have no fields.
            Ok((Vec::new(), Vec::new()))
        }
    }
}

fn generate_enum_variant_definitions(data_enum: &DataEnum) -> Result<Vec<TokenStream2>, Error> {
    let mut variant_names = Vec::new();

    for variant in &data_enum.variants {
        // Only support unit variants (no associated data).
        match &variant.fields {
            Fields::Unit => {
                let variant_name = variant.ident.to_string();
                variant_names.push(quote! { &#variant_name });
            }
            Fields::Named(_) => {
                return Err(Error::new_spanned(
                    variant,
                    "Type derive macro only supports unit enum variants, not struct variants",
                ));
            }
            Fields::Unnamed(_) => {
                return Err(Error::new_spanned(
                    variant,
                    "Type derive macro only supports unit enum variants, not tuple variants",
                ));
            }
        }
    }

    Ok(variant_names)
}
