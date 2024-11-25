use quote::quote;
use syn::DeriveInput;

use crate::cfg::FieldInfo;

/// Generate the `EVMProviderContext` implementation for the given struct.
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
        impl #impl_generics gadget_sdk::contexts::EVMProviderContext for #name #ty_generics #where_clause {
            type Network = alloy_network::Ethereum;
            type Transport = alloy_transport::BoxTransport;
            type Provider = alloy_provider::fillers::FillProvider<
                alloy_provider::fillers::RecommendedFiller,
                alloy_provider::RootProvider<Self::Transport>,
                Self::Transport,
                Self::Network,
            >;
            fn evm_provider(&self) -> impl core::future::Future<Output = Result<Self::Provider, alloy_transport::TransportError>> {
                    type Provider = alloy_provider::fillers::FillProvider<
                        alloy_provider::fillers::RecommendedFiller,
                        alloy_provider::RootProvider<alloy_transport::BoxTransport>,
                        alloy_transport::BoxTransport,
                        alloy_network::Ethereum,
                    >;
                    static PROVIDER: std::sync::OnceLock<Provider> = std::sync::OnceLock::new();
                    async {
                        match PROVIDER.get() {
                            Some(provider) => Ok(provider.clone()),
                            None => {
                                let rpc_url = #field_access.http_rpc_endpoint.as_str();
                                let provider = alloy_provider::ProviderBuilder::new()
                                    .with_recommended_fillers()
                                    .on_builtin(rpc_url)
                                    .await?;
                                PROVIDER
                                    .set(provider.clone())
                                    .map(|_| provider)
                                    .map_err(|_| {
                                        alloy_transport::TransportError::LocalUsageError(Box::new(
                                            std::io::Error::new(
                                                std::io::ErrorKind::Other,
                                                "Failed to set provider",
                                            ),
                                        ))
                                    })
                            }
                        }
                    }
                }
        }
    }
}
