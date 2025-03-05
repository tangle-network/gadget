use proc_macro2::Ident;
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

    let network_ty_ident = Ident::new(&format!("__{}Network", name), name.span());
    let provider_ty_ident = Ident::new(&format!("__{}Provider", name), name.span());

    quote! {
        type #network_ty_ident = ::blueprint_sdk::macros::ext::evm::alloy_network::Ethereum;
        type #provider_ty_ident = ::blueprint_sdk::macros::ext::evm::alloy_provider::fillers::FillProvider<
            ::blueprint_sdk::macros::ext::evm::alloy_provider::fillers::JoinFill<
                ::blueprint_sdk::macros::ext::evm::alloy_provider::Identity,
                <#network_ty_ident as ::blueprint_sdk::macros::ext::evm::alloy_provider::fillers::RecommendedFillers>::RecommendedFillers,
            >,
            ::blueprint_sdk::macros::ext::evm::alloy_provider::RootProvider,
            #network_ty_ident,
        >;

        #[automatically_derived]
        #[::blueprint_sdk::macros::ext::async_trait::async_trait]
        impl #impl_generics ::blueprint_sdk::macros::ext::contexts::instrumented_evm_client::EvmInstrumentedClientContext for #name #ty_generics #where_clause {
            async fn evm_client(&self) -> ::blueprint_sdk::macros::ext::contexts::instrumented_evm_client::InstrumentedClient {
                ::blueprint_sdk::macros::ext::contexts::instrumented_evm_client::InstrumentedClient::new(
                    &#field_access.http_rpc_endpoint,
                ).await.expect("Failed to create EVM client")
            }
        }
    }
}
