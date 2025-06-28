use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DataEnum, DeriveInput, Error, Fields};

/// Main entry point for the ReplyError derive macro.
pub(crate) fn derive_reply_error(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);

    match derive_reply_error_impl(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn derive_reply_error_impl(input: DeriveInput) -> Result<TokenStream2, Error> {
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let expanded = match &input.data {
        Data::Enum(data_enum) => {
            let error_variants = generate_error_definitions(data_enum)?;

            quote! {
                impl #impl_generics ::zlink::introspect::ReplyError for #name #ty_generics #where_clause {
                    const VARIANTS: &'static [&'static ::zlink::idl::Error<'static>] = &[
                        #(#error_variants),*
                    ];
                }
            }
        }
        Data::Struct(_) => {
            return Err(Error::new_spanned(
                input,
                "ReplyError derive macro only supports enums, not structs",
            ));
        }
        Data::Union(_) => {
            return Err(Error::new_spanned(
                input,
                "ReplyError derive macro only supports enums, not unions",
            ));
        }
    };

    Ok(expanded)
}

fn generate_error_definitions(data_enum: &DataEnum) -> Result<Vec<TokenStream2>, Error> {
    let mut error_variants = Vec::new();

    for variant in &data_enum.variants {
        let variant_name = variant.ident.to_string();

        match &variant.fields {
            Fields::Unit => {
                let error_variant = quote! {
                    &::zlink::idl::Error::new(#variant_name, &[])
                };
                error_variants.push(error_variant);
            }
            Fields::Named(fields) => {
                let (field_statics, field_refs) =
                    generate_field_definitions_for_named_variant(&variant.ident, fields)?;

                let error_variant = quote! {
                    &{
                        #(#field_statics)*

                        static FIELD_REFS: &[&::zlink::idl::Field<'static>] = &[
                            #(#field_refs),*
                        ];

                        ::zlink::idl::Error::new(#variant_name, FIELD_REFS)
                    }
                };
                error_variants.push(error_variant);
            }
            Fields::Unnamed(fields) => {
                if fields.unnamed.len() != 1 {
                    return Err(Error::new_spanned(
                        variant,
                        "ReplyError derive macro only supports tuple variants with exactly one field",
                    ));
                }

                let field_type = &fields.unnamed.first().unwrap().ty;
                let error_variant = quote! {
                    &{
                        match <#field_type as ::zlink::introspect::Type>::TYPE {
                            ::zlink::idl::Type::Object(fields) => {
                                let ::zlink::idl::List::Borrowed(field_slice) = fields else {
                                    panic!("Owned List not supported in const context")
                                };
                                ::zlink::idl::Error::new(#variant_name, field_slice)
                            }
                            _ => panic!("Tuple variant field type must have Type::Object"),
                        }
                    }
                };
                error_variants.push(error_variant);
            }
        }
    }

    Ok(error_variants)
}

fn generate_field_definitions_for_named_variant(
    variant_ident: &syn::Ident,
    fields: &syn::FieldsNamed,
) -> Result<(Vec<TokenStream2>, Vec<TokenStream2>), Error> {
    let mut field_statics = Vec::new();
    let mut field_refs = Vec::new();

    for field in &fields.named {
        let field_name = field
            .ident
            .as_ref()
            .ok_or_else(|| Error::new_spanned(field, "Field must have a name"))?;

        let field_type = &field.ty;
        let field_name_str = field_name.to_string();
        let static_name = quote::format_ident!(
            "FIELD_{}_{}",
            variant_ident.to_string().to_uppercase(),
            field_name.to_string().to_uppercase()
        );

        let field_static = quote! {
            static #static_name: ::zlink::idl::Field<'static> =
                ::zlink::idl::Field::new(
                    #field_name_str,
                    <#field_type as ::zlink::introspect::Type>::TYPE
                );
        };

        let field_ref = quote! { &#static_name };

        field_statics.push(field_static);
        field_refs.push(field_ref);
    }

    Ok((field_statics, field_refs))
}
