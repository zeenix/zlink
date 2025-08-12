use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashSet;
use syn::{punctuated::Punctuated, Error, FnArg, Pat, Type};

use super::{
    types::{ArgInfo, MethodAttrs},
    utils::{
        collect_used_type_params, convert_to_single_lifetime, extract_param_rename_attr,
        is_option_type_syn, parse_return_type, snake_case_to_pascal_case, type_contains_lifetime,
    },
};

pub(super) fn generate_method_impl(
    method: &mut syn::TraitItemFn,
    interface_name: &str,
    trait_generics: &syn::Generics,
    method_attrs: &MethodAttrs,
    crate_path: &TokenStream,
) -> Result<TokenStream, Error> {
    let method_name = &method.sig.ident;
    let method_name_str = method_name.to_string();

    let converted_name = snake_case_to_pascal_case(&method_name_str);
    let actual_method_name = method_attrs.rename.as_deref().unwrap_or(&converted_name);

    // Build the full method path: interface.method
    let method_path = format!("{interface_name}.{actual_method_name}");

    // Parse method arguments (skip &mut self)
    let has_explicit_lifetimes = method.sig.generics.lifetimes().next().is_some();

    // Extract data before mutable borrow
    let method_generics = method.sig.generics.clone();
    let method_output = method.sig.output.clone();

    // Process all method arguments in a single pass
    let arg_infos = parse_method_arguments(method, has_explicit_lifetimes)?;

    // Extract the data we need from the processed arguments
    let arg_names: Vec<_> = arg_infos.iter().map(|info| info.name).collect();
    let has_any_lifetime = arg_infos.iter().any(|info| info.has_lifetime);

    // Check for incompatible attributes
    if method_attrs.is_streaming && method_attrs.is_oneway {
        return Err(Error::new_spanned(
            &method.sig,
            "method cannot be both streaming (`more`) and oneway (`oneway`)",
        ));
    }

    // Parse return type
    let (reply_type, error_type) = if method_attrs.is_oneway {
        // For oneway methods, we don't parse the return type - just use dummy values
        // since we don't use them in the generated code
        (syn::parse_quote!(()), syn::parse_quote!(#crate_path::Error))
    } else {
        parse_return_type(&method_output, method_attrs.is_streaming)?
    };

    // Generate the method parameters as an Option
    let (params_struct_def, params_init) = generate_method_params(
        &arg_infos,
        &arg_names,
        &method_generics,
        trait_generics,
        has_any_lifetime,
        has_explicit_lifetimes,
    );

    // Common method call setup
    let method_call_setup = quote! {
        #params_struct_def
        #params_init

        #[derive(::serde::Serialize, ::core::fmt::Debug)]
        struct MethodCall<T> {
            method: &'static str,
            #[serde(skip_serializing_if = "Option::is_none")]
            parameters: Option<T>,
        }

        let method_call = MethodCall {
            method: #method_path,
            parameters: params,
        };
    };

    let out_params_extract = match &reply_type {
        Type::Tuple(tuple) if tuple.elems.is_empty() => {
            // Unit type ()
            quote!(Ok(Ok(())))
        }
        _ => {
            quote!(match reply.into_parameters() {
                Some(params) => Ok(Ok(params)),
                None => Err(#crate_path::Error::MissingParameters),
            })
        }
    };

    // Generate return type and implementation based on method attributes
    let (return_type, implementation) = if method_attrs.is_oneway {
        generate_oneway_method(method_call_setup, crate_path)
    } else if method_attrs.is_streaming {
        generate_streaming_method(
            method_call_setup,
            &reply_type,
            &error_type,
            out_params_extract,
            crate_path,
        )
    } else {
        generate_regular_method(
            method_call_setup,
            &reply_type,
            &error_type,
            out_params_extract,
            crate_path,
        )
    };

    // Generate the method implementation using the original signature but with new return type and
    // body
    let mut method_sig = method.sig.clone();
    method_sig.output = syn::parse2(quote! { -> #return_type })?;

    Ok(quote! {
        #method_sig
        {
            #implementation
        }
    })
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

            // Extract parameter rename attribute
            let serialized_name = extract_param_rename_attr(&mut pat_type.attrs)
                .ok()
                .flatten();

            // Check if the type is optional
            let is_optional = is_option_type_syn(ty);

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
                is_optional,
                has_lifetime,
                serialized_name,
            }))
        })
        .collect()
}

fn generate_method_params(
    arg_infos: &[ArgInfo<'_>],
    arg_names: &[&syn::Ident],
    method_generics: &syn::Generics,
    trait_generics: &syn::Generics,
    has_any_lifetime: bool,
    has_explicit_lifetimes: bool,
) -> (TokenStream, TokenStream) {
    if !arg_names.is_empty() {
        // Collect which type parameters are actually used in method arguments
        let mut used_type_params = HashSet::new();
        for info in arg_infos {
            collect_used_type_params(&info.ty_for_params, &mut used_type_params);
        }

        // Include only used trait generics and method generics for Params struct (without bounds)
        let mut combined_generics: Punctuated<syn::GenericParam, syn::Token![,]> =
            Punctuated::new();

        // Add trait generics that are actually used (without bounds)
        for param in &trait_generics.params {
            match param {
                syn::GenericParam::Type(type_param) => {
                    if used_type_params.contains(&type_param.ident.to_string()) {
                        let mut clean_param = type_param.clone();
                        clean_param.bounds.clear();
                        combined_generics.push(syn::GenericParam::Type(clean_param));
                    }
                }
                other => combined_generics.push(other.clone()),
            }
        }

        // Add method generics that are actually used (without bounds)
        for param in &method_generics.params {
            match param {
                syn::GenericParam::Type(type_param) => {
                    if used_type_params.contains(&type_param.ident.to_string()) {
                        let mut clean_param = type_param.clone();
                        clean_param.bounds.clear();
                        combined_generics.push(syn::GenericParam::Type(clean_param));
                    }
                }
                other => combined_generics.push(other.clone()),
            }
        }

        // Add lifetime if needed
        let generics_decl = if !combined_generics.is_empty() {
            quote! { <#combined_generics> }
        } else if has_any_lifetime && !has_explicit_lifetimes {
            quote! { <'__proxy_params> }
        } else {
            quote! {}
        };

        // Generate struct fields with optional serde attributes
        let struct_fields = arg_infos.iter().map(|info| {
            let name = info.name;
            let ty = &info.ty_for_params;

            let serde_attrs = if let Some(ref renamed) = info.serialized_name {
                if info.is_optional {
                    quote! {
                        #[serde(rename = #renamed, skip_serializing_if = "Option::is_none")]
                    }
                } else {
                    quote! {
                        #[serde(rename = #renamed)]
                    }
                }
            } else if info.is_optional {
                quote! {
                    #[serde(skip_serializing_if = "Option::is_none")]
                }
            } else {
                quote! {}
            };

            quote! {
                #serde_attrs
                #name: #ty
            }
        });

        // Add where clause with bounds from method's where clause for used type parameters
        let params_where_clause = build_params_where_clause(method_generics, &used_type_params);

        let struct_def = quote! {
            #[derive(::serde::Serialize, ::core::fmt::Debug)]
            struct Params #generics_decl
            #params_where_clause
            {
                #(#struct_fields,)*
            }
        };

        let init = quote! {
            let params = Some(Params {
                #(#arg_names,)*
            });
        };

        (struct_def, init)
    } else {
        (
            quote! {},
            quote! {
                // No parameters for methods without arguments
                let params: Option<()> = None;
            },
        )
    }
}

fn build_params_where_clause(
    method_generics: &syn::Generics,
    used_type_params: &HashSet<String>,
) -> Option<syn::WhereClause> {
    match &method_generics.where_clause {
        None => None,
        Some(method_where_clause) => {
            let mut where_predicates = syn::punctuated::Punctuated::new();

            for predicate in &method_where_clause.predicates {
                let syn::WherePredicate::Type(type_predicate) = predicate else {
                    continue;
                };
                let syn::Type::Path(type_path) = &type_predicate.bounded_ty else {
                    continue;
                };
                if type_path.path.segments.len() != 1 {
                    continue;
                }
                let type_name = &type_path.path.segments[0].ident;
                if used_type_params.contains(&type_name.to_string()) {
                    where_predicates.push(predicate.clone());
                }
            }

            if !where_predicates.is_empty() {
                Some(syn::WhereClause {
                    where_token: syn::token::Where::default(),
                    predicates: where_predicates,
                })
            } else {
                None
            }
        }
    }
}

fn generate_oneway_method(
    method_call_setup: TokenStream,
    crate_path: &TokenStream,
) -> (TokenStream, TokenStream) {
    let return_type = quote! {
        #crate_path::Result<()>
    };
    let implementation = quote! {
        #method_call_setup

        let call = #crate_path::Call::new(method_call).set_oneway(true);
        self.send_call(&call).await
    };
    (return_type, implementation)
}

fn generate_streaming_method(
    method_call_setup: TokenStream,
    reply_type: &Type,
    error_type: &Type,
    out_params_extract: TokenStream,
    crate_path: &TokenStream,
) -> (TokenStream, TokenStream) {
    let return_type = quote! {
        #crate_path::Result<
            impl ::futures_util::stream::Stream<
                Item = #crate_path::Result<::core::result::Result<#reply_type, #error_type>>
            >
        >
    };
    let implementation = quote! {
        #method_call_setup

        let call = #crate_path::Call::new(method_call).set_more(true);
        self.send_call(&call).await?;

        let stream = #crate_path::connection::chain::ReplyStream::new(
            self.read_mut(),
            |conn| conn.receive_reply::<#reply_type, #error_type>(),
            1,
        );

        use ::futures_util::stream::{Stream, StreamExt};
        Ok(stream.map(|result| {
            match result {
                Ok(Ok(reply)) => #out_params_extract,
                Ok(Err(error)) => Ok(Err(error)),
                Err(err) => Err(err),
            }
        }))
    };
    (return_type, implementation)
}

fn generate_regular_method(
    method_call_setup: TokenStream,
    reply_type: &Type,
    error_type: &Type,
    out_params_extract: TokenStream,
    crate_path: &TokenStream,
) -> (TokenStream, TokenStream) {
    let return_type = quote! {
        #crate_path::Result<::core::result::Result<#reply_type, #error_type>>
    };
    let implementation = quote! {
        #method_call_setup

        let call = #crate_path::Call::new(method_call);
        match self.call_method::<_, #reply_type, #error_type>(&call).await? {
            Ok(reply) => #out_params_extract,
            Err(error) => Ok(Err(error)),
        }
    };
    (return_type, implementation)
}
