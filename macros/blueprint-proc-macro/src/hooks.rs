use std::collections::BTreeMap;

use gadget_blueprint_proc_macro_core::{ServiceRegistrationHook, ServiceRequestHook};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Parse, ForeignItemFn, Token};

mod kw {
    syn::custom_keyword!(evm);
}

pub(crate) fn registration_hook_impl(
    args: &HookArgs,
    input: &ForeignItemFn,
) -> syn::Result<TokenStream> {
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
        .map(crate::job::type_to_field_type)
        .collect::<syn::Result<Vec<_>>>()?;

    let hook = match args {
        HookArgs::Evm(c) => ServiceRegistrationHook::Evm(c.value()),
    };

    let hook_json = serde_json::to_string(&hook).map_err(|err| {
        syn::Error::new_spanned(input, format!("failed to serialize hook: {err}"))
    })?;
    let hook_params = serde_json::to_string(&params).map_err(|err| {
        syn::Error::new_spanned(input, format!("failed to serialize hook: {err}"))
    })?;

    let gen = quote! {
        #[doc(hidden)]
        #[doc = "The registration hook for the service"]
        #[automatically_derived]
        pub const REGISTRATION_HOOK: &str = #hook_json;
        #[doc(hidden)]
        #[doc = "The registration hook parameters for the service"]
        #[automatically_derived]
        pub const REGISTRATION_HOOK_PARAMS: &str = #hook_params;
    };

    Ok(gen.into())
}

pub(crate) fn request_hook_impl(
    args: &HookArgs,
    input: &ForeignItemFn,
) -> syn::Result<TokenStream> {
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
        .map(crate::job::type_to_field_type)
        .collect::<syn::Result<Vec<_>>>()?;

    let hook = match args {
        HookArgs::Evm(c) => ServiceRequestHook::Evm(c.value()),
    };

    let hook_json = serde_json::to_string(&hook).map_err(|err| {
        syn::Error::new_spanned(input, format!("failed to serialize hook: {err}"))
    })?;
    let hook_params = serde_json::to_string(&params).map_err(|err| {
        syn::Error::new_spanned(input, format!("failed to serialize hook: {err}"))
    })?;

    let gen = quote! {
        #[doc(hidden)]
        #[doc = "The request hook for the service"]
        #[automatically_derived]
        pub const REQUEST_HOOK: &str = #hook_json;
        #[doc(hidden)]
        #[doc = "The request hook parameters for the service"]
        #[automatically_derived]
        pub const REQUEST_HOOK_PARAMS: &str = #hook_params;
    };

    Ok(gen.into())
}

pub(crate) enum HookArgs {
    Evm(syn::LitStr),
}

impl Parse for HookArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::evm) {
            let _ = input.parse::<kw::evm>()?;
            let _ = input.parse::<Token![=]>()?;
            let evm = input.parse::<syn::LitStr>()?;
            return Ok(HookArgs::Evm(evm));
        }

        Err(syn::Error::new(
            input.span(),
            "expected `evm = \"<contract-name>\"`",
        ))
    }
}
