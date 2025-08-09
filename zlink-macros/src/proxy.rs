use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse2, Error, ItemTrait, Lit, TraitItem};

mod method_impl;
mod types;
mod utils;

use method_impl::generate_method_impl;
use types::MethodAttrs;
use utils::build_combined_where_clause;

pub(crate) fn proxy(attr: TokenStream, input: TokenStream) -> TokenStream {
    match proxy_impl(attr, input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error(),
    }
}

fn proxy_impl(attr: TokenStream, input: TokenStream) -> Result<TokenStream, Error> {
    let mut trait_def = parse2::<ItemTrait>(input)?;

    // Parse the interface name from the attribute
    let interface_name = parse_interface_name(&attr, &trait_def)?;

    // Validate trait definition
    validate_trait(&trait_def)?;

    let trait_name = &trait_def.ident;
    let generics = &trait_def.generics;
    let where_clause = &trait_def.generics.where_clause;

    // Generate the implementation for Connection
    let methods = trait_def
        .items
        .iter_mut()
        .filter_map(|item| match item {
            TraitItem::Fn(method) => {
                let method_attrs = MethodAttrs::extract(&mut method.attrs).ok()?;
                Some(generate_method_impl(
                    method,
                    &interface_name,
                    generics,
                    &method_attrs,
                ))
            }
            _ => None,
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Build impl generics combining trait generics with socket generic
    let mut impl_generics = generics.clone();
    impl_generics.params.push(syn::parse_quote!(S));

    // Create trait generics without bounds for impl line
    let mut trait_generics_no_bounds = generics.clone();
    for param in &mut trait_generics_no_bounds.params {
        if let syn::GenericParam::Type(type_param) = param {
            type_param.bounds.clear(); // Remove inline bounds for impl line
        }
    }

    // Build where clause combining existing constraints with socket constraint and trait bounds
    let combined_where_clause = Some(build_combined_where_clause(
        where_clause.clone(),
        syn::parse_quote!(S: ::zlink::connection::socket::Socket),
        generics,
    ));

    let output = quote! {
        #trait_def

        impl #impl_generics #trait_name #trait_generics_no_bounds for ::zlink::Connection<S>
        #combined_where_clause
        {
            #(#methods)*
        }
    };

    Ok(output)
}

fn parse_interface_name(attr: &TokenStream, trait_def: &ItemTrait) -> Result<String, Error> {
    if attr.is_empty() {
        return Err(Error::new_spanned(
            trait_def,
            "proxy macro requires interface name, e.g. #[proxy(\"org.example.Interface\")]",
        ));
    }

    let interface_lit: Lit = parse2(attr.clone())?;
    match interface_lit {
        Lit::Str(lit_str) => Ok(lit_str.value()),
        _ => Err(Error::new_spanned(
            &interface_lit,
            "interface name must be a string literal",
        )),
    }
}

fn validate_trait(trait_def: &ItemTrait) -> Result<(), Error> {
    if !trait_def.items.is_empty()
        && trait_def
            .items
            .iter()
            .any(|item| !matches!(item, TraitItem::Fn(_)))
    {
        return Err(Error::new_spanned(
            trait_def,
            "proxy macro only supports traits with method definitions",
        ));
    }
    Ok(())
}
