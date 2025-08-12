use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse2, Error, ItemTrait, Lit, TraitItem};

mod chain_extension;
mod chain_method;
mod method_impl;
mod types;
mod utils;

use chain_extension::generate_chain_extension_method;
use chain_method::generate_chain_method;
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

    // Generate implementations for each method
    let mut methods = Vec::new();
    let mut chain_method_traits = Vec::new();
    let mut chain_method_impls = Vec::new();
    let mut chain_extension_methods = Vec::new();
    let mut chain_extension_impls = Vec::new();

    // Process methods first before we need trait_def references
    for item in &mut trait_def.items {
        if let TraitItem::Fn(method) = item {
            // Extract attributes once to avoid multiple mutable borrows
            let method_attrs = MethodAttrs::extract(&mut method.attrs)?;

            // Generate chain extension method
            let (extension_method, extension_impl) = generate_chain_extension_method(
                method,
                &interface_name,
                &trait_def.generics,
                &method_attrs,
            )?;
            if !extension_method.is_empty() {
                chain_extension_methods.push(extension_method);
            }
            if !extension_impl.is_empty() {
                chain_extension_impls.push(extension_impl);
            }

            // Generate regular method implementation
            let method_impl =
                generate_method_impl(method, &interface_name, &trait_def.generics, &method_attrs)?;
            methods.push(method_impl);

            // Generate chain method
            let (chain_trait, chain_impl) =
                generate_chain_method(method, &interface_name, &trait_def.generics, &method_attrs)?;
            if !chain_trait.is_empty() {
                chain_method_traits.push(chain_trait);
            }
            if !chain_impl.is_empty() {
                chain_method_impls.push(chain_impl);
            }
        }
    }

    // Build the output components
    let trait_output = build_trait_output(&mut trait_def, &chain_method_traits)?;
    let impl_output = build_impl_output(
        &trait_def.ident,
        &trait_def.generics,
        &trait_def.generics.where_clause,
        &methods,
        &chain_method_impls,
    );
    let chain_extension_trait_output = build_chain_extension_trait(
        &trait_def.ident,
        &chain_extension_methods,
        &chain_extension_impls,
    );

    Ok(quote! {
        #trait_output
        #impl_output
        #chain_extension_trait_output
    })
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

fn build_trait_output(
    trait_def: &mut ItemTrait,
    chain_method_traits: &[TokenStream],
) -> Result<TokenStream, Error> {
    // Add the Socket associated type to the trait
    trait_def.items.push(syn::parse2(quote! {
        /// The socket type used for the connection.
        type Socket: ::zlink::connection::socket::Socket;
    })?);

    // Add chain method signatures to the trait definition
    for chain_trait in chain_method_traits {
        trait_def.items.push(syn::parse2(chain_trait.clone())?);
    }

    Ok(quote! {
        #[allow(async_fn_in_trait)]
        #trait_def
    })
}

fn build_impl_output(
    trait_name: &syn::Ident,
    generics: &syn::Generics,
    where_clause: &Option<syn::WhereClause>,
    methods: &[TokenStream],
    chain_method_impls: &[TokenStream],
) -> TokenStream {
    // Build impl generics combining trait generics with socket generic
    let mut impl_generics = generics.clone();
    impl_generics.params.push(syn::parse_quote!(S));

    // Create trait generics without bounds for impl line
    let mut trait_generics_no_bounds = generics.clone();
    for param in &mut trait_generics_no_bounds.params {
        if let syn::GenericParam::Type(type_param) = param {
            type_param.bounds.clear();
        }
    }

    // Build where clause combining existing constraints with socket constraint and trait bounds
    let combined_where_clause = Some(build_combined_where_clause(
        where_clause.clone(),
        syn::parse_quote!(S: ::zlink::connection::socket::Socket),
        generics,
    ));

    quote! {
        impl #impl_generics #trait_name #trait_generics_no_bounds for ::zlink::Connection<S>
        #combined_where_clause
        {
            type Socket = S;

            #(#methods)*
            #(#chain_method_impls)*
        }
    }
}

fn build_chain_extension_trait(
    trait_name: &syn::Ident,
    chain_extension_methods: &[TokenStream],
    chain_extension_impls: &[TokenStream],
) -> TokenStream {
    if chain_extension_methods.is_empty() {
        return quote! {};
    }

    let chain_trait_name = syn::Ident::new(&format!("{trait_name}Chain"), trait_name.span());

    quote! {
        /// Extension trait for adding proxy calls to any chain.
        ///
        /// This trait provides methods to add proxy calls to a chain of method calls.
        pub trait #chain_trait_name<'c, S, ReplyParams, ReplyError>
        where
            S: ::zlink::connection::socket::Socket,
            ReplyParams: ::serde::Deserialize<'c> + ::core::fmt::Debug,
            ReplyError: ::serde::Deserialize<'c> + ::core::fmt::Debug,
        {
            #(#chain_extension_methods)*
        }

        impl<'c, S, ReplyParams, ReplyError> #chain_trait_name<'c, S, ReplyParams, ReplyError>
            for ::zlink::connection::chain::Chain<'c, S, ReplyParams, ReplyError>
        where
            S: ::zlink::connection::socket::Socket,
            ReplyParams: ::serde::Deserialize<'c> + ::core::fmt::Debug,
            ReplyError: ::serde::Deserialize<'c> + ::core::fmt::Debug,
        {
            #(#chain_extension_impls)*
        }
    }
}
