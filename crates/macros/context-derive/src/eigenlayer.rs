use quote::quote;
use syn::DeriveInput;

use crate::cfg::FieldInfo;

/// Generate the `EigenlayerContext` implementation for the given struct.
#[allow(clippy::too_many_lines)]
pub fn generate_context_impl(
    DeriveInput {
        ident: name,
        generics,
        ..
    }: DeriveInput,
    config_field: FieldInfo,
) -> proc_macro2::TokenStream {
    let field_access = match config_field {
        FieldInfo::Named(ident) => quote! { self.#ident },
        FieldInfo::Unnamed(index) => quote! { self.#index },
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let config_ty = quote! { ::blueprint_sdk::contexts::eigenlayer::EigenlayerClient };

    quote! {
        impl #impl_generics ::blueprint_sdk::contexts::eigenlayer::EigenlayerContext for #name #ty_generics #where_clause {
            async fn eigenlayer_client(&self) -> std::result::Result<#config_ty, ::blueprint_sdk::clients::Error> {
                static CLIENT: std::sync::OnceLock<#config_ty> = std::sync::OnceLock::new();
                match CLIENT.get() {
                    Some(client) => Ok(client.clone()),
                    None => {
                        let client = ::blueprint_sdk::contexts::eigenlayer::EigenlayerContext::eigenlayer_client(&#field_access).await?;
                        CLIENT.set(client.clone()).map(|_| client).map_err(|_| {
                            ::blueprint_sdk::clients::Error::msg("Failed to set client")
                        })
                    }
                }
            }
        }
    }
}
