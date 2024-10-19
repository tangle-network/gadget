use quote::quote;
use syn::DeriveInput;

use crate::cfg::FieldInfo;

/// Generate the `EigenlayerContext` implementation for the given struct.
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

    let mut generics = generics.clone();
    generics.params.push(syn::parse_quote!(NodeConfig));
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics gadget_sdk::ctx::EigenlayerContext for #name #ty_generics #where_clause {
            type Config = NodeConfig;

            fn eigenlayer_provider(&self) -> impl core::future::Future<Output = Result<alloy_provider::RootProvider<Self::Config::TH>, alloy_transport::TransportError>> {
                async {
                    let http_endpoint = #field_access.eigenlayer_http_endpoint.clone();
                    let provider = alloy_provider::ProviderBuilder::new()
                        .with_recommended_fillers()
                        .on_http(http_endpoint.parse().unwrap())
                        .root()
                        .clone()
                        .boxed();
                    Ok(provider)
                }
            }

            fn eigenlayer_avs_registry_reader(&self) -> impl core::future::Future<Output = Result<eigensdk::client_avsregistry::reader::AvsRegistryChainReader, std::io::Error>> {
                async {
                    let http_endpoint = #field_access.eigenlayer_http_endpoint.clone();
                    let registry_coordinator_addr = #field_access.eigenlayer_registry_coordinator_addr;
                    let operator_state_retriever_addr = #field_access.eigenlayer_operator_state_retriever_addr;

                    Ok(eigensdk::client_avsregistry::reader::AvsRegistryChainReader::new(
                        eigensdk::logging::get_test_logger(),
                        registry_coordinator_addr,
                        operator_state_retriever_addr,
                        http_endpoint,
                    ).await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?)
                }
            }

            fn eigenlayer_avs_registry_writer(&self) -> impl core::future::Future<Output = Result<eigensdk::client_avsregistry::writer::AvsRegistryChainWriter, std::io::Error>> {
                async {
                    let http_endpoint = #field_access.eigenlayer_http_endpoint.clone();
                    let private_key = #field_access.eigenlayer_private_key.clone();
                    let registry_coordinator_addr = #field_access.eigenlayer_registry_coordinator_addr;
                    let operator_state_retriever_addr = #field_access.eigenlayer_operator_state_retriever_addr;

                    Ok(eigensdk::client_avsregistry::writer::AvsRegistryChainWriter::build_avs_registry_chain_writer(
                        eigensdk::logging::get_test_logger(),
                        http_endpoint,
                        private_key,
                        registry_coordinator_addr,
                        operator_state_retriever_addr,
                    ).await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?)
                }
            }

            fn eigenlayer_operator_info_service(&self) -> impl core::future::Future<Output = Result<eigensdk::services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceInMemory, std::io::Error>> {
                async {
                    let avs_registry_reader = self.eigenlayer_avs_registry_reader().await?;
                    let ws_endpoint = #field_access.eigenlayer_ws_endpoint.clone();

                    Ok(eigensdk::services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceInMemory::new(
                        eigensdk::logging::get_test_logger(),
                        avs_registry_reader,
                        ws_endpoint,
                    ).await)
                }
            }

            fn eigenlayer_bls_aggregation_service(&self) -> impl core::future::Future<Output = Result<eigensdk::services_blsaggregation::bls_agg::BlsAggregatorService, std::io::Error>> {
                async {
                    let avs_registry_reader = self.eigenlayer_avs_registry_reader().await?;
                    let operator_info_service = self.eigenlayer_operator_info_service().await?;

                    let avs_registry_service = eigensdk::services_avsregistry::chaincaller::AvsRegistryServiceChainCaller::new(
                        avs_registry_reader,
                        operator_info_service,
                    );

                    Ok(eigensdk::services_blsaggregation::bls_agg::BlsAggregatorService::new(avs_registry_service))
                }
            }
        }
    }
}
