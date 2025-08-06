use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DataEnum, DeriveInput, Error, Fields};

use crate::utils;

use super::shared;

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

    let type_comments = utils::extract_doc_comments(&input.attrs);
    let type_comment_objects = shared::generate_comment_objects(&type_comments, &crate_path);

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
                    #crate_path::idl::CustomObject::new(#name_str, FIELD_REFS, &[#(#type_comment_objects),*])
                )
            })
        }
        Data::Enum(data_enum) => {
            let variant_refs = generate_enum_variant_definitions(data_enum, &crate_path)?;

            quote!({
                static VARIANT_REFS: &[&#crate_path::idl::EnumVariant<'static>] = &[
                    #(#variant_refs),*
                ];

                #crate_path::idl::CustomType::Enum(
                   #crate_path::idl::CustomEnum::new(#name_str, VARIANT_REFS, &[#(#type_comment_objects),*])
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
    shared::generate_field_definitions(fields, crate_path, None)
}

fn generate_enum_variant_definitions(
    data_enum: &DataEnum,
    crate_path: &TokenStream2,
) -> Result<Vec<TokenStream2>, Error> {
    shared::generate_enum_variant_definitions(data_enum, crate_path)
}
