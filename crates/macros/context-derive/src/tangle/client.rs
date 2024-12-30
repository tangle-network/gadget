use quote::quote;
use syn::DeriveInput;

use crate::cfg::FieldInfo;

/// Generate the `TangleClientContext` implementation for the given struct.
pub fn generate_context_impl(
    DeriveInput {
        ident: name,
        generics,
        ..
    }: DeriveInput,
    config_field: FieldInfo,
) -> proc_macro2::TokenStream {
    let field_access_config = match config_field {
        FieldInfo::Named(ident) => quote! { self.#ident },
        FieldInfo::Unnamed(index) => quote! { self.#index },
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let config_ty = quote! { ::gadget_macros::ext::contexts::tangle::TangleClient };

    quote! {
        #[::gadget_macros::ext::async_trait::async_trait]
        impl #impl_generics ::gadget_macros::ext::contexts::tangle::TangleClientContext for #name #ty_generics #where_clause {
            async fn tangle_client(&self) -> Result<#config_ty, ::gadget_macros::ext::clients::Error> {
                use ::gadget_macros::ext::tangle::tangle_subxt::subxt;

                static CLIENT: std::sync::OnceLock<#config_ty> = std::sync::OnceLock::new();
                match CLIENT.get() {
                    Some(client) => Ok(client.clone()),
                    None => {
                        let client = #config_ty::new(#field_access_config.clone()).await?;
                        CLIENT.set(client.clone()).map(|_| client).map_err(|_| {
                            ::gadget_macros::ext::clients::Error::Other(String::from("Failed to set client"))
                        })
                    }
                }
            }
        }
    }
}
