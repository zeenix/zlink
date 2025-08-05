use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Attribute, Error, GenericArgument, PathArguments, Type};

/// Parse the crate path from attributes, defaulting to `::zlink`.
///
/// Looks for `#[zlink(crate = "...")]` attribute and uses the specified crate path.
/// If no such attribute is found, defaults to `::zlink`.
///
/// # Examples
///
/// ```ignore
/// #[derive(Type)]
/// #[zlink(crate = "crate")]
/// struct MyStruct;
/// ```
pub(crate) fn parse_crate_path(attrs: &[Attribute]) -> Result<TokenStream2, Error> {
    for attr in attrs {
        if attr.path().is_ident("zlink") {
            let mut result = None;
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("crate") {
                    let value = meta.value()?;
                    let lit_str: syn::LitStr = value.parse()?;
                    let crate_path = lit_str.value();
                    result = Some(syn::parse_str(&crate_path)?);
                } else {
                    // Skip unknown attributes by consuming their values
                    let _ = meta.value()?;
                    let _: syn::Expr = meta.input.parse()?;
                }
                Ok(())
            })?;

            if let Some(path) = result {
                return Ok(path);
            }
        }
    }
    // Default to ::zlink
    Ok(quote! { ::zlink })
}

/// Extract doc comments from attributes.
///
/// Each `#[doc = "..."]` attribute becomes a single comment string.
pub(crate) fn extract_doc_comments(attrs: &[Attribute]) -> Vec<String> {
    let mut comments = Vec::new();

    for attr in attrs {
        if attr.path().is_ident("doc") {
            // Try different parsing methods
            if let Ok(lit_str) = attr.parse_args::<syn::LitStr>() {
                comments.push(lit_str.value());
            } else if let syn::Meta::NameValue(meta_name_value) = &attr.meta {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(lit_str),
                    ..
                }) = &meta_name_value.value
                {
                    comments.push(lit_str.value());
                }
            }
        }
    }

    comments
}

/// Recursively removes all lifetimes from a type.
pub(crate) fn remove_lifetimes_from_type(ty: &Type) -> Type {
    match ty {
        Type::Reference(type_ref) => Type::Reference(syn::TypeReference {
            and_token: type_ref.and_token,
            lifetime: None,
            mutability: type_ref.mutability,
            elem: Box::new(remove_lifetimes_from_type(&type_ref.elem)),
        }),
        Type::Path(type_path) => {
            let mut new_type_path = type_path.clone();
            if let Some(ref mut qself) = new_type_path.qself {
                qself.ty = Box::new(remove_lifetimes_from_type(&qself.ty));
            }
            for segment in &mut new_type_path.path.segments {
                match &mut segment.arguments {
                    PathArguments::AngleBracketed(args) => {
                        // Filter out lifetime arguments and recursively process type arguments
                        let mut new_args = syn::punctuated::Punctuated::new();
                        for arg in args.args.iter() {
                            match arg {
                                GenericArgument::Type(ty) => {
                                    new_args.push(GenericArgument::Type(
                                        remove_lifetimes_from_type(ty),
                                    ));
                                }
                                GenericArgument::Lifetime(_) => {
                                    // Replace with elided lifetime '_
                                    new_args.push(GenericArgument::Lifetime(syn::Lifetime::new(
                                        "'_",
                                        proc_macro2::Span::call_site(),
                                    )));
                                }
                                other => {
                                    new_args.push(other.clone());
                                }
                            }
                        }
                        args.args = new_args;
                    }
                    PathArguments::Parenthesized(args) => {
                        for input in &mut args.inputs {
                            *input = remove_lifetimes_from_type(input);
                        }
                        if let syn::ReturnType::Type(_, ref mut output) = args.output {
                            *output = Box::new(remove_lifetimes_from_type(output));
                        }
                    }
                    PathArguments::None => {}
                }
            }
            Type::Path(new_type_path)
        }
        Type::Tuple(type_tuple) => {
            let mut new_type_tuple = type_tuple.clone();
            for elem in &mut new_type_tuple.elems {
                *elem = remove_lifetimes_from_type(elem);
            }
            Type::Tuple(new_type_tuple)
        }
        Type::Array(type_array) => {
            let mut new_type_array = type_array.clone();
            new_type_array.elem = Box::new(remove_lifetimes_from_type(&type_array.elem));
            Type::Array(new_type_array)
        }
        Type::Slice(type_slice) => {
            let mut new_type_slice = type_slice.clone();
            new_type_slice.elem = Box::new(remove_lifetimes_from_type(&type_slice.elem));
            Type::Slice(new_type_slice)
        }
        Type::Ptr(type_ptr) => {
            let mut new_type_ptr = type_ptr.clone();
            new_type_ptr.elem = Box::new(remove_lifetimes_from_type(&type_ptr.elem));
            Type::Ptr(new_type_ptr)
        }
        Type::Group(type_group) => {
            let mut new_type_group = type_group.clone();
            new_type_group.elem = Box::new(remove_lifetimes_from_type(&type_group.elem));
            Type::Group(new_type_group)
        }
        Type::Paren(type_paren) => {
            let mut new_type_paren = type_paren.clone();
            new_type_paren.elem = Box::new(remove_lifetimes_from_type(&type_paren.elem));
            Type::Paren(new_type_paren)
        }
        // For types that don't contain other types or lifetimes, return as-is
        Type::BareFn(_)
        | Type::ImplTrait(_)
        | Type::Infer(_)
        | Type::Macro(_)
        | Type::Never(_)
        | Type::TraitObject(_)
        | Type::Verbatim(_) => ty.clone(),

        // Handle any new types added to syn that we haven't covered
        _ => ty.clone(),
    }
}
