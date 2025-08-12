use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, FnArg, Pat};

use super::{
    types::{ArgInfo, MethodAttrs},
    utils::{convert_to_single_lifetime, snake_case_to_pascal_case, type_contains_lifetime},
};

pub(super) fn generate_chain_extension_method(
    method: &mut syn::TraitItemFn,
    interface_name: &str,
    _trait_generics: &syn::Generics,
    method_attrs: &MethodAttrs,
    crate_path: &TokenStream,
) -> Result<(TokenStream, TokenStream), Error> {
    let method_name_str = method.sig.ident.to_string();
    let method_ident = method.sig.ident.clone();

    // Check for explicit lifetimes early
    let has_explicit_lifetimes = method.sig.generics.lifetimes().next().is_some();

    // Skip chain extension methods for oneway and streaming methods
    if method_attrs.is_oneway || method_attrs.is_streaming {
        return Ok((quote! {}, quote! {}));
    }

    let converted_name = snake_case_to_pascal_case(&method_name_str);
    let actual_method_name = method_attrs.rename.as_deref().unwrap_or(&converted_name);
    let method_path = format!("{interface_name}.{actual_method_name}");

    // Extract data we need before mutable borrow
    let method_generic_params = method.sig.generics.params.clone();
    let method_where_clause = method.sig.generics.where_clause.clone();

    // Parse method arguments (skip &mut self)
    let arg_infos = parse_method_arguments(method, has_explicit_lifetimes)?;
    let arg_names: Vec<_> = arg_infos.iter().map(|info| info.name).collect();
    let has_any_lifetime = arg_infos.iter().any(|info| info.has_lifetime);

    // Generate method signature with all method generics
    let generics = build_method_generics(
        &method_generic_params,
        has_any_lifetime,
        has_explicit_lifetimes,
    );

    // Generate where clause combining lifetime bound with method's where clause
    let combined_where_clause = build_combined_where_clause(&method_where_clause);

    // Generate parameter list for the chain extension method
    let param_fields: Vec<_> = arg_infos
        .iter()
        .map(|info| {
            let name = info.name;
            let ty = &info.ty_for_params;
            quote! { #name: #ty }
        })
        .collect();

    if arg_infos.is_empty() {
        generate_no_params_method(&method_ident, &method_path, crate_path)
    } else {
        generate_with_params_method(
            &method_ident,
            &method_path,
            generics,
            combined_where_clause,
            param_fields,
            arg_names,
            &method_generic_params,
            &method_where_clause,
            has_any_lifetime,
            has_explicit_lifetimes,
            crate_path,
        )
    }
}

fn parse_method_arguments<'a>(
    method: &'a mut syn::TraitItemFn,
    has_explicit_lifetimes: bool,
) -> Result<Vec<ArgInfo<'a>>, Error> {
    method
        .sig
        .inputs
        .iter_mut()
        .skip(1)
        .filter_map(|arg| {
            let FnArg::Typed(pat_type) = arg else {
                return None;
            };
            let Pat::Ident(pat_ident) = &*pat_type.pat else {
                return None;
            };

            let name = &pat_ident.ident;
            let ty = &pat_type.ty;

            // Only convert to single lifetime if there are no explicit lifetimes
            let ty_for_params = if has_explicit_lifetimes {
                (**ty).clone()
            } else {
                convert_to_single_lifetime(ty)
            };

            // Check if this argument has lifetimes
            let has_lifetime = type_contains_lifetime(&ty_for_params);

            Some(Ok(ArgInfo {
                name,
                ty_for_params,
                has_lifetime,
                is_optional: false,
                serialized_name: None,
            }))
        })
        .collect()
}

fn build_method_generics(
    method_generic_params: &syn::punctuated::Punctuated<syn::GenericParam, syn::Token![,]>,
    has_any_lifetime: bool,
    has_explicit_lifetimes: bool,
) -> TokenStream {
    if !method_generic_params.is_empty() {
        if has_any_lifetime && !has_explicit_lifetimes {
            quote! { <'__proxy_params, #method_generic_params> }
        } else {
            quote! { <#method_generic_params> }
        }
    } else if has_any_lifetime && !has_explicit_lifetimes {
        quote! { <'__proxy_params> }
    } else {
        quote! {}
    }
}

fn build_combined_where_clause(method_where_clause: &Option<syn::WhereClause>) -> TokenStream {
    let mut combined_where_predicates =
        syn::punctuated::Punctuated::<syn::WherePredicate, syn::Token![,]>::new();
    if let Some(method_where) = method_where_clause {
        for predicate in &method_where.predicates {
            combined_where_predicates.push(predicate.clone());
        }
    }

    if !combined_where_predicates.is_empty() {
        quote! { where #combined_where_predicates }
    } else {
        quote! {}
    }
}

fn generate_no_params_method(
    method_name: &syn::Ident,
    method_path: &str,
    crate_path: &TokenStream,
) -> Result<(TokenStream, TokenStream), Error> {
    let trait_method = quote! {
        /// Add a #method_name call to this chain.
        fn #method_name(
            self,
        ) -> #crate_path::Result<#crate_path::connection::chain::Chain<'c, S, ReplyParams, ReplyError>>;
    };

    let impl_method = quote! {
        fn #method_name(
            self,
        ) -> #crate_path::Result<#crate_path::connection::chain::Chain<'c, S, ReplyParams, ReplyError>> {
            let call = #crate_path::Call::new({
                #[derive(::serde::Serialize, ::core::fmt::Debug)]
                #[serde(tag = "method")]
                enum MethodWrapper {
                    #[serde(rename = #method_path)]
                    Method,
                }
                MethodWrapper::Method
            });
            self.append(&call)
        }
    };

    Ok((trait_method, impl_method))
}

#[allow(clippy::too_many_arguments)]
fn generate_with_params_method(
    method_name: &syn::Ident,
    method_path: &str,
    generics: TokenStream,
    combined_where_clause: TokenStream,
    param_fields: Vec<TokenStream>,
    arg_names: Vec<&syn::Ident>,
    method_generic_params: &syn::punctuated::Punctuated<syn::GenericParam, syn::Token![,]>,
    method_where_clause: &Option<syn::WhereClause>,
    has_any_lifetime: bool,
    has_explicit_lifetimes: bool,
    crate_path: &TokenStream,
) -> Result<(TokenStream, TokenStream), Error> {
    let trait_method = quote! {
        /// Add a #method_name call to this chain.
        fn #method_name #generics(
            self,
            #(#param_fields,)*
        ) -> #crate_path::Result<#crate_path::connection::chain::Chain<'c, S, ReplyParams, ReplyError>>
        #combined_where_clause;
    };

    // Generate unique struct names for this method to avoid conflicts
    let params_struct_name = syn::Ident::new(
        &format!(
            "{}Params",
            snake_case_to_pascal_case(&method_name.to_string())
        ),
        method_name.span(),
    );
    let wrapper_enum_name = syn::Ident::new(
        &format!(
            "{}Wrapper",
            snake_case_to_pascal_case(&method_name.to_string())
        ),
        method_name.span(),
    );

    // Build generics without bounds for usage in type paths
    let struct_generics_without_bounds = build_struct_generics_without_bounds(
        method_generic_params,
        has_any_lifetime,
        has_explicit_lifetimes,
    );

    // Build struct where clause adding Serialize and Debug bounds for generic parameters
    let struct_where = build_struct_where_clause(method_generic_params, method_where_clause);

    let impl_method = quote! {
        fn #method_name #generics(
            self,
            #(#param_fields,)*
        ) -> #crate_path::Result<#crate_path::connection::chain::Chain<'c, S, ReplyParams, ReplyError>>
        #combined_where_clause
        {
            let call = #crate_path::Call::new({
                #[derive(::serde::Serialize, ::core::fmt::Debug)]
                struct #params_struct_name #generics
                #struct_where
                {
                    #(#param_fields,)*
                }

                #[derive(::serde::Serialize, ::core::fmt::Debug)]
                #[serde(tag = "method", content = "parameters")]
                enum #wrapper_enum_name #struct_generics_without_bounds
                #struct_where
                {
                    #[serde(rename = #method_path)]
                    Method(#params_struct_name #struct_generics_without_bounds),
                }

                #wrapper_enum_name::Method(#params_struct_name {
                    #(#arg_names,)*
                })
            });
            self.append(&call)
        }
    };

    Ok((trait_method, impl_method))
}

fn build_struct_generics_without_bounds(
    method_generic_params: &syn::punctuated::Punctuated<syn::GenericParam, syn::Token![,]>,
    has_any_lifetime: bool,
    has_explicit_lifetimes: bool,
) -> TokenStream {
    if !method_generic_params.is_empty() {
        let generic_names: Vec<_> = method_generic_params
            .iter()
            .map(|param| match param {
                syn::GenericParam::Type(type_param) => {
                    let name = &type_param.ident;
                    quote! { #name }
                }
                syn::GenericParam::Lifetime(lifetime_param) => {
                    let lifetime = &lifetime_param.lifetime;
                    quote! { #lifetime }
                }
                syn::GenericParam::Const(const_param) => {
                    let name = &const_param.ident;
                    quote! { #name }
                }
            })
            .collect();

        if has_any_lifetime && !has_explicit_lifetimes {
            quote! { <'__proxy_params, #(#generic_names),*> }
        } else {
            quote! { <#(#generic_names),*> }
        }
    } else if has_any_lifetime && !has_explicit_lifetimes {
        quote! { <'__proxy_params> }
    } else {
        quote! {}
    }
}

fn build_struct_where_clause(
    method_generic_params: &syn::punctuated::Punctuated<syn::GenericParam, syn::Token![,]>,
    method_where_clause: &Option<syn::WhereClause>,
) -> Option<syn::WhereClause> {
    let mut predicates = syn::punctuated::Punctuated::<syn::WherePredicate, syn::Token![,]>::new();

    // Add Serialize and Debug bounds for all type parameters
    for param in method_generic_params {
        if let syn::GenericParam::Type(type_param) = param {
            let ident = &type_param.ident;
            predicates.push(syn::parse_quote!(#ident: ::serde::Serialize));
            predicates.push(syn::parse_quote!(#ident: ::core::fmt::Debug));
        }
    }

    // Add existing method where clause predicates
    if let Some(method_where) = method_where_clause {
        for predicate in &method_where.predicates {
            predicates.push(predicate.clone());
        }
    }

    if predicates.is_empty() {
        None
    } else {
        Some(syn::WhereClause {
            where_token: syn::token::Where::default(),
            predicates,
        })
    }
}
