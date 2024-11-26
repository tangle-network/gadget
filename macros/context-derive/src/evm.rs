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
        type #network_ty_ident = ::gadget_sdk::ext::alloy_network::Ethereum;
        type #transport_ty_ident = ::gadget_sdk::ext::alloy_transport::BoxTransport;
        type #provider_ty_ident = ::gadget_sdk::ext::alloy_provider::fillers::FillProvider<
            ::gadget_sdk::ext::alloy_provider::fillers::JoinFill<
                ::gadget_sdk::ext::alloy_provider::Identity,
                <#network_ty_ident as ::gadget_sdk::ext::alloy_provider::fillers::RecommendedFillers>::RecomendedFillers,
            >,
            ::gadget_sdk::ext::alloy_provider::RootProvider<#transport_ty_ident>,
            #transport_ty_ident,
            #network_ty_ident,
        >;

        #[automatically_derived]
        impl #impl_generics ::gadget_sdk::contexts::EVMProviderContext for #name #ty_generics #where_clause {
            type Network = #network_ty_ident;
            type Transport = #transport_ty_ident;
            type Provider = #provider_ty_ident;
            fn evm_provider(&self) -> impl core::future::Future<Output = Result<Self::Provider, ::gadget_sdk::ext::alloy_transport::TransportError>> {
                static PROVIDER: std::sync::OnceLock<#provider_ty_ident> = std::sync::OnceLock::new();
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
