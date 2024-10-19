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

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics gadget_sdk::ctx::EigenlayerContext for #name #ty_generics #where_clause {
            fn avs_registry_reader(&self) -> impl core::future::Future<Output = Result<eigensdk::client_avsregistry::reader::AvsRegistryChainReader, std::io::Error>> {
                async {
                    let http_rpc_endpoint = #field_access.http_rpc_endpoint.clone();
                    let contract_addrs = match &#field_access.eigenlayer_contract_addrs {
                        Some(addrs) => addrs,
                        None => return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Contract addresses not found")),
                    };
                    let registry_coordinator_addr = contract_addrs.registry_coordinator_addr;
                    let operator_state_retriever_addr = contract_addrs.operator_state_retriever_addr;

                    Ok(eigensdk::client_avsregistry::reader::AvsRegistryChainReader::new(
                        eigensdk::logging::get_test_logger(),
                        registry_coordinator_addr,
                        operator_state_retriever_addr,
                        http_rpc_endpoint,
                    ).await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?)
                }
            }

            fn avs_registry_writer(&self, private_key: String) -> impl core::future::Future<Output = Result<eigensdk::client_avsregistry::writer::AvsRegistryChainWriter, std::io::Error>> {
                async {
                    let http_rpc_endpoint = #field_access.http_rpc_endpoint.clone();
                    let contract_addrs = match &#field_access.eigenlayer_contract_addrs {
                        Some(addrs) => addrs,
                        None => return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Contract addresses not found")),
                    };
                    let registry_coordinator_addr = contract_addrs.registry_coordinator_addr;
                    let operator_state_retriever_addr = contract_addrs.operator_state_retriever_addr;

                    Ok(eigensdk::client_avsregistry::writer::AvsRegistryChainWriter::build_avs_registry_chain_writer(
                        eigensdk::logging::get_test_logger(),
                        http_rpc_endpoint,
                        private_key,
                        registry_coordinator_addr,
                        operator_state_retriever_addr,
                    ).await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?)
                }
            }

            fn operator_info_service_in_memory(&self) -> impl core::future::Future<Output = Result<eigensdk::services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceInMemory, std::io::Error>> {
                async {
                    let avs_registry_reader = self.avs_registry_reader().await?;
                    let ws_endpoint = #field_access.ws_rpc_endpoint.clone();

                    Ok(eigensdk::services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceInMemory::new(
                        eigensdk::logging::get_test_logger(),
                        avs_registry_reader,
                        ws_endpoint,
                    ).await)
                }
            }

            fn avs_registry_service_chain_caller_in_memory(&self) -> impl core::future::Future<Output = Result<
                eigensdk::services_avsregistry::chaincaller::AvsRegistryServiceChainCaller<
                    eigensdk::client_avsregistry::reader::AvsRegistryChainReader,
                    eigensdk::services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceInMemory
                >, std::io::Error>> {
                async {
                    let avs_registry_reader = self.avs_registry_reader().await?;
                    let operator_info_service = self.operator_info_service_in_memory().await?;

                    Ok(eigensdk::services_avsregistry::chaincaller::AvsRegistryServiceChainCaller::new(
                        avs_registry_reader,
                        operator_info_service,
                    ))
                }
            }

            fn bls_aggregation_service_in_memory(&self) -> impl core::future::Future<Output = Result<eigensdk::services_blsaggregation::bls_agg::BlsAggregatorService<
                eigensdk::services_avsregistry::chaincaller::AvsRegistryServiceChainCaller<
                    eigensdk::client_avsregistry::reader::AvsRegistryChainReader,
                    eigensdk::services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceInMemory
                >
            >, std::io::Error>> {
                async {
                    let avs_registry_service = self.avs_registry_service_chain_caller_in_memory().await?;

                    Ok(eigensdk::services_blsaggregation::bls_agg::BlsAggregatorService::new(avs_registry_service))
                }
            }
        }
    }
}
