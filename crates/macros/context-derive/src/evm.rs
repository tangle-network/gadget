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
    let transport_ty_ident = Ident::new(&format!("__{}Transport", name), name.span());
    let provider_ty_ident = Ident::new(&format!("__{}Provider", name), name.span());

    quote! {
        type #network_ty_ident = ::gadget_macros::ext::evm::alloy_network::Ethereum;
        type #transport_ty_ident = ::gadget_macros::ext::evm::alloy_transport::BoxTransport;
        type #provider_ty_ident = ::gadget_macros::ext::evm::alloy_provider::fillers::FillProvider<
            ::gadget_macros::ext::evm::alloy_provider::fillers::JoinFill<
                ::gadget_macros::ext::evm::alloy_provider::Identity,
                <#network_ty_ident as ::gadget_macros::ext::evm::alloy_provider::fillers::RecommendedFillers>::RecomendedFillers,
            >,
            ::gadget_macros::ext::evm::alloy_provider::RootProvider<#transport_ty_ident>,
            #transport_ty_ident,
            #network_ty_ident,
        >;

        #[automatically_derived]
        impl #impl_generics ::gadget_macros::ext::contexts::instrumented_evm_client::EvmInstrumentedClientContext for #name #ty_generics #where_clause {
            fn evm_client(&self) -> ::gadget_macros::ext::contexts::instrumented_evm_client::InstrumentedClient {
                ::gadget_macros::ext::contexts::instrumented_evm_client::InstrumentedClient::new(
                    #field_access.http_rpc_endpoint.clone(),
                ).expect("Failed to create EVM client")
            }
        }
    }
}
