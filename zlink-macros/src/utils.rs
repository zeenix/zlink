use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Attribute, Error};

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
