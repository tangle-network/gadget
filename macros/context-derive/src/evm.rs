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
        impl #impl_generics gadget_sdk::ctx::EVMProviderContext for #name #ty_generics #where_clause {
            type Network = alloy_network::Ethereum;
            type Transport = alloy_transport::BoxTransport;
            type Provider = alloy_provider::fillers::FillProvider<
                alloy_provider::fillers::RecommendedFiller,
                alloy_provider::RootProvider<Self::Transport>,
                Self::Transport,
                Self::Network,
            >;
            fn evm_provider(&self) -> impl core::future::Future<Output = Result<Self::Provider, alloy_transport::TransportError>> {
                // TODO: think about caching the provider
                // see: https://github.com/webb-tools/gadget/issues/320
                async {
                    let rpc_url = #field_access.rpc_endpoint.as_str();
                    let provider = alloy_provider::ProviderBuilder::new()
                        .with_recommended_fillers()
                        .on_builtin(rpc_url)
                        .await?;
                    Ok(provider)
                }
            }
        }
    }
}
