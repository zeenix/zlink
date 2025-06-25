use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, FieldsNamed, FieldsUnnamed};

/// Main entry point for the TypeInfo derive macro.
pub(crate) fn derive_type_info(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);

    match derive_type_info_impl(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn derive_type_info_impl(input: DeriveInput) -> Result<TokenStream2, Error> {
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Check for unsupported attributes.
    check_attributes(&input.attrs)?;

    // Only support structs.
    let fields = match &input.data {
        Data::Struct(data_struct) => &data_struct.fields,
        Data::Enum(_) => {
            return Err(Error::new_spanned(
                input,
                "TypeInfo derive macro only supports structs, not enums",
            ));
        }
        Data::Union(_) => {
            return Err(Error::new_spanned(
                input,
                "TypeInfo derive macro only supports structs, not unions",
            ));
        }
    };

    let (field_statics, field_refs) = generate_field_definitions(name, fields)?;

    let expanded = quote! {
        impl #impl_generics ::zlink::idl::TypeInfo for #name #ty_generics #where_clause {
            const TYPE_INFO: &'static ::zlink::idl::Type<'static> = &{
                #(#field_statics)*

                static FIELD_REFS: &[&::zlink::idl::Field<'static>] = &[
                    #(#field_refs),*
                ];

                ::zlink::idl::Type::Struct(::zlink::idl::List::Borrowed(FIELD_REFS))
            };
        }
    };

    Ok(expanded)
}

fn check_attributes(attrs: &[syn::Attribute]) -> Result<(), Error> {
    for attr in attrs {
        if attr.path().is_ident("zlink") {
            return Err(Error::new_spanned(
                attr,
                "zlink attributes are not yet supported on TypeInfo derive",
            ));
        }
    }
    Ok(())
}

fn generate_field_definitions(
    _struct_name: &syn::Ident,
    fields: &Fields,
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

                // Check for unsupported field attributes.
                check_attributes(&field.attrs)?;
                let field_type = &field.ty;
                let field_name_str = field_name.to_string();
                let static_name =
                    quote::format_ident!("FIELD_{}", field_name.to_string().to_uppercase());

                let field_static = quote! {
                    static #static_name: ::zlink::idl::Field<'static> =
                        ::zlink::idl::Field::new(
                            #field_name_str,
                            <#field_type as ::zlink::idl::TypeInfo>::TYPE_INFO
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
