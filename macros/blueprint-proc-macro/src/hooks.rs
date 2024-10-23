use std::collections::BTreeMap;

use proc_macro::TokenStream;
use quote::quote;
use syn::ForeignItemFn;

pub(crate) fn registration_hook_impl(input: &ForeignItemFn) -> syn::Result<TokenStream> {
    let syn::ReturnType::Default = &input.sig.output else {
        return Err(syn::Error::new_spanned(
            &input.sig.output,
            "hooks does not return any value",
        ));
    };

    let mut param_types = BTreeMap::new();
    for input in &input.sig.inputs {
        if let syn::FnArg::Typed(arg) = input {
            if let syn::Pat::Ident(pat_ident) = &*arg.pat {
                let ident = &pat_ident.ident;
                let ty = &*arg.ty;
                let added = param_types.insert(ident.clone(), ty.clone());
                if added.is_some() {
                    return Err(syn::Error::new_spanned(
                        ident,
                        "tried to add the same field twice",
                    ));
                }
            }
        }
    }

    let params = param_types
        .values()
        .map(crate::shared::type_to_field_type)
        .collect::<syn::Result<Vec<_>>>()?;

    let hook_params = serde_json::to_string(&params).map_err(|err| {
        syn::Error::new_spanned(input, format!("failed to serialize hook: {err}"))
    })?;

    let gen = quote! {
        #[doc(hidden)]
        #[doc = "The registration hook parameters for the service"]
        #[automatically_derived]
        pub const REGISTRATION_HOOK_PARAMS: &str = #hook_params;
    };

    Ok(gen.into())
}

pub(crate) fn request_hook_impl(input: &ForeignItemFn) -> syn::Result<TokenStream> {
    let syn::ReturnType::Default = &input.sig.output else {
        return Err(syn::Error::new_spanned(
            &input.sig.output,
            "hooks does not return any value",
        ));
    };

    let mut param_types = BTreeMap::new();
    for input in &input.sig.inputs {
        if let syn::FnArg::Typed(arg) = input {
            if let syn::Pat::Ident(pat_ident) = &*arg.pat {
                let ident = &pat_ident.ident;
                let ty = &*arg.ty;
                let added = param_types.insert(ident.clone(), ty.clone());
                if added.is_some() {
                    return Err(syn::Error::new_spanned(
                        ident,
                        "tried to add the same field twice",
                    ));
                }
            }
        }
    }

    let params = param_types
        .values()
        .map(crate::shared::type_to_field_type)
        .collect::<syn::Result<Vec<_>>>()?;

    let hook_params = serde_json::to_string(&params).map_err(|err| {
        syn::Error::new_spanned(input, format!("failed to serialize hook: {err}"))
    })?;

    let gen = quote! {
        #[doc(hidden)]
        #[doc = "The request hook parameters for the service"]
        #[automatically_derived]
        pub const REQUEST_HOOK_PARAMS: &str = #hook_params;
    };

    Ok(gen.into())
}
