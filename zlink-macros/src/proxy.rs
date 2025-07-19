use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse2, spanned::Spanned, Attribute, Error, Expr, FnArg, GenericArgument, ItemTrait, Lifetime,
    Lit, Meta, MetaNameValue, Pat, PathArguments, ReturnType, TraitItem, Type, TypeReference,
};

pub(crate) fn proxy(attr: TokenStream, input: TokenStream) -> TokenStream {
    match proxy_impl(attr, input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error(),
    }
}

fn proxy_impl(attr: TokenStream, input: TokenStream) -> Result<TokenStream, Error> {
    let trait_def = parse2::<ItemTrait>(input)?;

    // Parse the interface name from the attribute
    let interface_name = if attr.is_empty() {
        return Err(Error::new_spanned(
            &trait_def,
            "proxy macro requires interface name, e.g. #[proxy(\"org.example.Interface\")]",
        ));
    } else {
        let interface_lit: Lit = parse2(attr)?;
        match interface_lit {
            Lit::Str(lit_str) => lit_str.value(),
            _ => {
                return Err(Error::new_spanned(
                    &interface_lit,
                    "interface name must be a string literal",
                ))
            }
        }
    };

    // Validate that this is a trait definition
    if !trait_def.items.is_empty()
        && trait_def
            .items
            .iter()
            .any(|item| !matches!(item, TraitItem::Fn(_)))
    {
        return Err(Error::new_spanned(
            &trait_def,
            "proxy macro only supports traits with method definitions",
        ));
    }

    let trait_name = &trait_def.ident;
    let generics = &trait_def.generics;
    let where_clause = &trait_def.generics.where_clause;

    // Generate the implementation for Connection
    let methods = trait_def
        .items
        .iter()
        .filter_map(|item| {
            if let TraitItem::Fn(method) = item {
                Some(generate_method_impl(method, &interface_name))
            } else {
                None
            }
        })
        .collect::<Result<Vec<_>, _>>()?;

    let output = quote! {
        #trait_def

        impl<S> #trait_name #generics for ::zlink::Connection<S>
        where
            S: ::zlink::connection::socket::Socket,
            #where_clause
        {
            #(#methods)*
        }
    };

    Ok(output)
}

fn generate_method_impl(
    method: &syn::TraitItemFn,
    interface_name: &str,
) -> Result<TokenStream, Error> {
    let method_name = &method.sig.ident;
    let method_name_str = method_name.to_string();

    // Look for #[zlink(rename = "...")] attribute, otherwise convert snake_case to PascalCase
    let method_name_override = extract_method_rename(&method.attrs)?;
    let converted_name = snake_case_to_pascal_case(&method_name_str);
    let actual_method_name = method_name_override.as_deref().unwrap_or(&converted_name);

    // Build the full method path: interface.method
    let method_path = format!("{interface_name}.{actual_method_name}");

    // Collect all lifetimes from method generics
    let _method_lifetimes: Vec<_> = method.sig.generics.lifetimes().collect();

    // Parse method arguments (skip &mut self)
    let has_explicit_lifetimes = method.sig.generics.lifetimes().next().is_some();

    let args = method
        .sig
        .inputs
        .iter()
        .skip(1)
        .filter_map(|arg| {
            if let FnArg::Typed(pat_type) = arg {
                if let Pat::Ident(pat_ident) = &*pat_type.pat {
                    let name = &pat_ident.ident;
                    let ty = &pat_type.ty;

                    // Only convert to single lifetime if there are no explicit lifetimes
                    let ty_for_params = if has_explicit_lifetimes {
                        (**ty).clone()
                    } else {
                        convert_to_single_lifetime(ty)
                    };

                    return Some(quote! {
                        #name: #ty_for_params
                    });
                }
            }
            None
        })
        .collect::<Vec<_>>();

    let arg_names = method
        .sig
        .inputs
        .iter()
        .skip(1)
        .filter_map(|arg| {
            if let FnArg::Typed(pat_type) = arg {
                if let Pat::Ident(pat_ident) = &*pat_type.pat {
                    return Some(&pat_ident.ident);
                }
            }
            None
        })
        .collect::<Vec<_>>();

    // Use the original method signature for the implementation
    let full_args = &method.sig.inputs;

    // Parse return type
    let (reply_type, error_type) = parse_return_type(&method.sig.output)?;

    // Generate the method parameters as an Option
    let (params_struct_def, params_init) = if !arg_names.is_empty() {
        // Check if we need a lifetime for the params struct
        let needs_lifetime = args.iter().any(|arg| {
            // Check if the arg contains any references by looking for '&' in the token stream
            arg.to_string().contains('&')
        });

        let lifetime_decl = if needs_lifetime {
            if has_explicit_lifetimes {
                // Use all the explicit lifetimes from the method
                let lifetimes: Vec<_> = method.sig.generics.lifetimes().collect();
                quote! { <#(#lifetimes),*> }
            } else {
                quote! { <'__proxy_params> }
            }
        } else {
            quote! {}
        };

        let struct_def = quote! {
            #[derive(::serde::Serialize, ::core::fmt::Debug)]
            struct Params #lifetime_decl {
                #(#args,)*
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
    };

    // Use the original method generics - don't modify them
    let method_generics = &method.sig.generics;

    Ok(quote! {
        async fn #method_name #method_generics (
            #full_args
        ) -> ::zlink::Result<::core::result::Result<#reply_type, #error_type>> {
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

            let call = ::zlink::Call::new(method_call);
            match self.call_method::<_, #reply_type, #error_type>(&call).await? {
                Ok(reply) => match reply.into_parameters() {
                    Some(params) => Ok(Ok(params)),
                    None => {
                        // Return an error for missing parameters
                        return Err(::zlink::Error::BufferOverflow);
                    },
                },
                Err(error) => Ok(Err(error)),
            }
        }
    })
}

fn extract_method_rename(attrs: &[Attribute]) -> Result<Option<String>, Error> {
    for attr in attrs {
        if attr.path().is_ident("zlink") {
            if let Meta::List(list) = &attr.meta {
                let args: MetaNameValue = syn::parse2(list.tokens.clone())?;
                if args.path.is_ident("rename") {
                    if let Expr::Lit(lit) = &args.value {
                        if let Lit::Str(lit_str) = &lit.lit {
                            return Ok(Some(lit_str.value()));
                        }
                    }
                }
            }
        }
    }
    Ok(None)
}

fn parse_return_type(output: &ReturnType) -> Result<(Type, Type), Error> {
    match output {
        ReturnType::Default => Err(Error::new_spanned(
            output,
            "proxy methods must have a return type",
        )),
        ReturnType::Type(_, ty) => {
            // Extract Result<Result<T, E>> or impl Future<Output = Result<Result<T, E>>>
            extract_nested_result_types(ty)
        }
    }
}

fn extract_nested_result_types(ty: &Type) -> Result<(Type, Type), Error> {
    match ty {
        Type::Path(type_path) => {
            // Direct Result<Result<T, E>>
            if let Some(segment) = type_path.path.segments.last() {
                if segment.ident == "Result" {
                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                            return extract_inner_result_types(inner_ty);
                        }
                    }
                }
            }
        }
        Type::ImplTrait(impl_trait) => {
            // impl Future<Output = Result<Result<T, E>>>
            for bound in &impl_trait.bounds {
                let trait_bound = match bound {
                    syn::TypeParamBound::Trait(trait_bound) => trait_bound,
                    _ => continue,
                };
                let segment = match trait_bound.path.segments.last() {
                    Some(segment) if segment.ident == "Future" => segment,
                    _ => continue,
                };
                let args = match &segment.arguments {
                    PathArguments::AngleBracketed(args) => args,
                    _ => continue,
                };
                for arg in &args.args {
                    match arg {
                        GenericArgument::AssocType(assoc) if assoc.ident == "Output" => {
                            return extract_nested_result_types(&assoc.ty);
                        }
                        _ => continue,
                    }
                }
            }
        }
        _ => {}
    }

    Err(Error::new_spanned(
        ty,
        "expected Result<Result<ReplyType, ErrorType>> or \
         impl Future<Output = Result<Result<ReplyType, ErrorType>>>",
    ))
}

fn extract_inner_result_types(ty: &Type) -> Result<(Type, Type), Error> {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Result" {
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    if args.args.len() == 2 {
                        if let (
                            Some(GenericArgument::Type(reply_ty)),
                            Some(GenericArgument::Type(error_ty)),
                        ) = (args.args.iter().next(), args.args.iter().nth(1))
                        {
                            return Ok((reply_ty.clone(), error_ty.clone()));
                        }
                    }
                }
            }
        }
    }

    Err(Error::new_spanned(
        ty,
        "expected inner Result<ReplyType, ErrorType>",
    ))
}

fn snake_case_to_pascal_case(input: &str) -> String {
    input
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
        })
        .collect()
}

// Convert any lifetime references to use our single '__proxy_params lifetime
fn convert_to_single_lifetime(ty: &Type) -> Type {
    match ty {
        Type::Reference(type_ref) => {
            let lifetime = if type_ref.lifetime.is_some() {
                // Replace any existing lifetime with our single lifetime
                Some(Lifetime::new("'__proxy_params", type_ref.and_token.span))
            } else {
                // Add our single lifetime for elided lifetimes
                Some(Lifetime::new("'__proxy_params", type_ref.and_token.span))
            };

            let elem = convert_to_single_lifetime(&type_ref.elem);

            Type::Reference(TypeReference {
                and_token: type_ref.and_token,
                lifetime,
                mutability: type_ref.mutability,
                elem: Box::new(elem),
            })
        }
        Type::Path(type_path) => {
            let mut new_path = type_path.clone();
            for segment in &mut new_path.path.segments {
                let PathArguments::AngleBracketed(args) = &mut segment.arguments else {
                    continue;
                };
                let mut new_args = args.clone();
                new_args.args = args
                    .args
                    .iter()
                    .map(|arg| match arg {
                        GenericArgument::Type(ty) => {
                            GenericArgument::Type(convert_to_single_lifetime(ty))
                        }
                        GenericArgument::Lifetime(_) => {
                            // Replace any lifetime with our single lifetime
                            GenericArgument::Lifetime(Lifetime::new("'__proxy_params", arg.span()))
                        }
                        _ => arg.clone(),
                    })
                    .collect();
                segment.arguments = PathArguments::AngleBracketed(new_args);
            }
            Type::Path(new_path)
        }
        Type::Slice(type_slice) => {
            let elem = convert_to_single_lifetime(&type_slice.elem);
            Type::Slice(syn::TypeSlice {
                bracket_token: type_slice.bracket_token,
                elem: Box::new(elem),
            })
        }
        Type::Array(type_array) => {
            let elem = convert_to_single_lifetime(&type_array.elem);
            Type::Array(syn::TypeArray {
                bracket_token: type_array.bracket_token,
                elem: Box::new(elem),
                semi_token: type_array.semi_token,
                len: type_array.len.clone(),
            })
        }
        Type::Tuple(type_tuple) => {
            let elems = type_tuple
                .elems
                .iter()
                .map(convert_to_single_lifetime)
                .collect();
            Type::Tuple(syn::TypeTuple {
                paren_token: type_tuple.paren_token,
                elems,
            })
        }
        _ => ty.clone(),
    }
}
