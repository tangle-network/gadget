use proc_macro::TokenStream;
use quote::quote;
use syn::ForeignItemFn;

pub(crate) fn registration_hook_impl(input: &ForeignItemFn) -> syn::Result<TokenStream> {
    let hook_params = generate_hook_params(input)?;

    let gen = quote! {
        #[doc(hidden)]
        #[doc = "The registration hook parameters for the service"]
        #[automatically_derived]
        pub const REGISTRATION_HOOK_PARAMS: &str = #hook_params;
    };

    Ok(gen.into())
}

fn generate_hook_params(input: &ForeignItemFn) -> syn::Result<String> {
    let syn::ReturnType::Default = &input.sig.output else {
        return Err(syn::Error::new_spanned(
            &input.sig.output,
            "hooks does not return any value",
        ));
    };

    let param_types = crate::shared::param_types(&input.sig)?;

    let mut params = Vec::new();
    for param_type in param_types.values() {
        let param_type = crate::shared::type_to_field_type(param_type)?;
        params.push(param_type.ty);
    }

    let hook_params = serde_json::to_string(&params).map_err(|err| {
        syn::Error::new_spanned(input, format!("failed to serialize hook: {err}"))
    })?;

    Ok(hook_params)
}

pub(crate) fn request_hook_impl(input: &ForeignItemFn) -> syn::Result<TokenStream> {
    let hook_params = generate_hook_params(input)?;

    let gen = quote! {
        #[doc(hidden)]
        #[doc = "The request hook parameters for the service"]
        #[automatically_derived]
        pub const REQUEST_HOOK_PARAMS: &str = #hook_params;
    };

    Ok(gen.into())
}
