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
    let field_access = match config_field {
        FieldInfo::Named(ident) => quote! { self.#ident },
        FieldInfo::Unnamed(index) => quote! { self.#index },
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics gadget_sdk::ctx::TangleClientContext for #name #ty_generics #where_clause {
            type Config = gadget_sdk::ext::subxt::PolkadotConfig;
            fn tangle_client(&self) -> impl core::future::Future<Output = Result<gadget_sdk::ext::subxt::OnlineClient<Self::Config>, gadget_sdk::ext::subxt::Error>> {
                use gadget_sdk::ext::subxt;

                type Config = subxt::PolkadotConfig;
                static CLIENT: std::sync::OnceLock<subxt::OnlineClient<Config>> = std::sync::OnceLock::new();
                async {
                    match CLIENT.get() {
                        Some(client) => Ok(client.clone()),
                        None => {
                            let rpc_url = #field_access.ws_rpc_endpoint.as_str();
                            let client = subxt::OnlineClient::from_url(rpc_url).await?;
                            CLIENT.set(client.clone()).map(|_| client).map_err(|_| {
                                subxt::Error::Io(std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    "Failed to set client",
                                ))
                            })
                        }
                    }
                }
            }
        }
    }
}
