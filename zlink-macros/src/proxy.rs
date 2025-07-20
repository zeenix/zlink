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
    let mut trait_def = parse2::<ItemTrait>(input)?;

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
        .iter_mut()
        .filter_map(|item| {
            if let TraitItem::Fn(method) = item {
                // Generate the method implementation (this also removes zlink attributes)
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
    method: &mut syn::TraitItemFn,
    interface_name: &str,
) -> Result<TokenStream, Error> {
    let method_name = &method.sig.ident;
    let method_name_str = method_name.to_string();

    // Look for #[zlink(rename = "...")] attribute, otherwise convert snake_case to PascalCase
    let method_name_override = extract_method_rename(&mut method.attrs)?;
    let converted_name = snake_case_to_pascal_case(&method_name_str);
    let actual_method_name = method_name_override.as_deref().unwrap_or(&converted_name);

    // Build the full method path: interface.method
    let method_path = format!("{interface_name}.{actual_method_name}");

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

fn extract_method_rename(attrs: &mut Vec<Attribute>) -> Result<Option<String>, Error> {
    let mut rename_value = None;
    let mut zlink_attr_index = None;

    for (i, attr) in attrs.iter().enumerate() {
        if !attr.path().is_ident("zlink") {
            continue;
        }

        let Meta::List(list) = &attr.meta else {
            continue;
        };

        let args: MetaNameValue = syn::parse2(list.tokens.clone())?;

        let lit_str = match (&args.path, &args.value) {
            (path, Expr::Lit(lit)) if path.is_ident("rename") => match &lit.lit {
                Lit::Str(lit_str) => lit_str,
                _ => continue,
            },
            _ => continue,
        };

        rename_value = Some(lit_str.value());
        zlink_attr_index = Some(i);

        break;
    }

    // Remove the zlink attribute if found
    if let Some(index) = zlink_attr_index {
        attrs.remove(index);
    }

    Ok(rename_value)
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
            let Some(segment) = type_path.path.segments.last() else {
                return Err(Error::new_spanned(
                    ty,
                    "expected Result<Result<ReplyType, ErrorType>> or \
                     impl Future<Output = Result<Result<ReplyType, ErrorType>>>",
                ));
            };

            if segment.ident != "Result" {
                return Err(Error::new_spanned(
                    ty,
                    "expected Result<Result<ReplyType, ErrorType>> or \
                     impl Future<Output = Result<Result<ReplyType, ErrorType>>>",
                ));
            }

            let PathArguments::AngleBracketed(args) = &segment.arguments else {
                return Err(Error::new_spanned(
                    ty,
                    "expected Result<Result<ReplyType, ErrorType>> or \
                     impl Future<Output = Result<Result<ReplyType, ErrorType>>>",
                ));
            };

            let Some(GenericArgument::Type(inner_ty)) = args.args.first() else {
                return Err(Error::new_spanned(
                    ty,
                    "expected Result<Result<ReplyType, ErrorType>> or \
                     impl Future<Output = Result<Result<ReplyType, ErrorType>>>",
                ));
            };

            extract_inner_result_types(inner_ty)
        }
        Type::ImplTrait(impl_trait) => {
            // impl Future<Output = Result<Result<T, E>>>
            for bound in &impl_trait.bounds {
                let syn::TypeParamBound::Trait(trait_bound) = bound else {
                    continue;
                };

                let segment = match trait_bound.path.segments.last() {
                    Some(segment) if segment.ident == "Future" => segment,
                    _ => continue,
                };

                let PathArguments::AngleBracketed(args) = &segment.arguments else {
                    continue;
                };

                for arg in &args.args {
                    let GenericArgument::AssocType(assoc) = arg else {
                        continue;
                    };

                    if assoc.ident == "Output" {
                        return extract_nested_result_types(&assoc.ty);
                    }
                }
            }

            Err(Error::new_spanned(
                ty,
                "expected Result<Result<ReplyType, ErrorType>> or \
                 impl Future<Output = Result<Result<ReplyType, ErrorType>>>",
            ))
        }
        _ => Err(Error::new_spanned(
            ty,
            "expected Result<Result<ReplyType, ErrorType>> or \
             impl Future<Output = Result<Result<ReplyType, ErrorType>>>",
        )),
    }
}

fn extract_inner_result_types(ty: &Type) -> Result<(Type, Type), Error> {
    let Type::Path(type_path) = ty else {
        return Err(Error::new_spanned(
            ty,
            "expected inner Result<ReplyType, ErrorType>",
        ));
    };

    let segment = match type_path.path.segments.last() {
        Some(segment) if segment.ident == "Result" => segment,
        _ => {
            return Err(Error::new_spanned(
                ty,
                "expected inner Result<ReplyType, ErrorType>",
            ))
        }
    };

    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return Err(Error::new_spanned(
            ty,
            "expected inner Result<ReplyType, ErrorType>",
        ));
    };

    match (args.args.get(0), args.args.get(1)) {
        (Some(GenericArgument::Type(reply_ty)), Some(GenericArgument::Type(error_ty)))
            if args.args.len() == 2 =>
        {
            Ok((reply_ty.clone(), error_ty.clone()))
        }
        _ => Err(Error::new_spanned(
            ty,
            "expected inner Result<ReplyType, ErrorType>",
        )),
    }
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
