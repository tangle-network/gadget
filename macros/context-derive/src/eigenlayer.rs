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

    quote! {
        use alloy_provider::Provider;
        use eigensdk::client_avsregistry::reader::AvsRegistryReader;

        #[async_trait::async_trait]
        impl #impl_generics gadget_sdk::ctx::EigenlayerContext for #name #ty_generics #where_clause {
            async fn avs_registry_reader(&self) -> Result<eigensdk::client_avsregistry::reader::AvsRegistryChainReader, std::io::Error> {
                let http_rpc_endpoint = #field_access.http_rpc_endpoint.clone();
                let gadget_sdk::config::ProtocolSpecificSettings::Eigenlayer(contract_addresses) = &#field_access.protocol_specific else {
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Expected Eigenlayer protocol"));
                };
                let registry_coordinator_address = contract_addresses.registry_coordinator_address;
                let operator_state_retriever_address = contract_addresses.operator_state_retriever_address;
                eigensdk::client_avsregistry::reader::AvsRegistryChainReader::new(
                    eigensdk::logging::get_test_logger(),
                    registry_coordinator_address,
                    operator_state_retriever_address,
                    http_rpc_endpoint,
                ).await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
            }

            async fn avs_registry_writer(&self, private_key: String) -> Result<eigensdk::client_avsregistry::writer::AvsRegistryChainWriter, std::io::Error> {
                let http_rpc_endpoint = #field_access.http_rpc_endpoint.clone();
                let gadget_sdk::config::ProtocolSpecificSettings::Eigenlayer(contract_addresses) = &#field_access.protocol_specific else {
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Expected Eigenlayer protocol"));
                };
                let registry_coordinator_address = contract_addresses.registry_coordinator_address;
                let operator_state_retriever_address = contract_addresses.operator_state_retriever_address;

                eigensdk::client_avsregistry::writer::AvsRegistryChainWriter::build_avs_registry_chain_writer(
                    eigensdk::logging::get_test_logger(),
                    http_rpc_endpoint,
                    private_key,
                    registry_coordinator_address,
                    operator_state_retriever_address,
                ).await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
            }

            async fn operator_info_service_in_memory(&self) -> Result<eigensdk::services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceInMemory, std::io::Error> {
                let avs_registry_reader = self.avs_registry_reader().await?;
                let ws_endpoint = #field_access.ws_rpc_endpoint.clone();

                Ok(eigensdk::services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceInMemory::new(
                    eigensdk::logging::get_test_logger(),
                    avs_registry_reader,
                    ws_endpoint,
                ).await)
            }

            async fn avs_registry_service_chain_caller_in_memory(&self) -> Result<
                eigensdk::services_avsregistry::chaincaller::AvsRegistryServiceChainCaller<
                    eigensdk::client_avsregistry::reader::AvsRegistryChainReader,
                    eigensdk::services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceInMemory
                >, std::io::Error> {
                let http_rpc_endpoint = #field_access.http_rpc_endpoint.clone();
                let avs_registry_reader = self.avs_registry_reader().await?;
                let operator_info_service = self.operator_info_service_in_memory().await?;

                let cancellation_token = tokio_util::sync::CancellationToken::new();
                let token_clone = cancellation_token.clone();
                let provider = alloy_provider::ProviderBuilder::new()
                    .with_recommended_fillers()
                    .on_http(http_rpc_endpoint.parse().map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?)
                    .root()
                    .clone()
                    .boxed();
                let current_block = provider.get_block_number()
                    .await
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                let operator_info_clone = operator_info_service.clone();

                tokio::task::spawn(async move {
                    operator_info_clone.start_service(&token_clone, 0, current_block).await
                });

                Ok(eigensdk::services_avsregistry::chaincaller::AvsRegistryServiceChainCaller::new(
                    avs_registry_reader,
                    operator_info_service,
                ))
            }

            async fn bls_aggregation_service_in_memory(&self) -> Result<eigensdk::services_blsaggregation::bls_agg::BlsAggregatorService<
                eigensdk::services_avsregistry::chaincaller::AvsRegistryServiceChainCaller<
                    eigensdk::client_avsregistry::reader::AvsRegistryChainReader,
                    eigensdk::services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceInMemory
                >
            >, std::io::Error> {
                let avs_registry_service = self.avs_registry_service_chain_caller_in_memory().await?;
                Ok(eigensdk::services_blsaggregation::bls_agg::BlsAggregatorService::new(avs_registry_service))
            }

            async fn get_operator_stake_in_quorums_at_block(
                &self,
                block_number: u32,
                quorum_numbers: Bytes,
            ) -> Result<Vec<Vec<eigensdk::utils::bindings::OperatorStateRetriever::Operator>>, std::io::Error> {
                let http_rpc_endpoint = #field_access.http_rpc_endpoint.clone();
                self.avs_registry_reader().await?.get_operators_stake_in_quorums_at_block(
                    block_number,
                    quorum_numbers,
                ).await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
            }

            // async fn get_operator_stake_in_quorums_at_current_block(
            //     &self,
            //     operator_id: FixedBytes<32>,
            // ) -> Result<HashMap<u8, U256>, std::io::Error> {
            //     let http_rpc_endpoint = #field_access.http_rpc_endpoint.clone();
            //     let contract_addresses = &gadget_sdk::config::ProtocolSpecificSettings::Eigenlayer.contract_addresses;
            //     eigensdk::client_avsregistry::reader::get_operator_stake_in_quorums_at_current_block(
            //         http_rpc_endpoint,
            //         contract_addresses,
            //         operator_id,
            //     ).await
            // }
            //
            // async fn get_operator_by_id(
            //     &self
            // ) -> Result<OperatorStateRetriever::Operator, std::io::Error> {
            //     let http_rpc_endpoint = #field_access.http_rpc_endpoint.clone();
            //     let contract_addresses = &gadget_sdk::config::ProtocolSpecificSettings::Eigenlayer.contract_addresses;
            //     eigensdk::client_avsregistry::reader::get_operator_by_id(
            //         http_rpc_endpoint,
            //         contract_addresses,
            //     ).await
            // }
            //
            // async fn get_operator_stake_history(
            //     &self,
            //     operator_id: FixedBytes<32>,
            //     quorum_number: u8,
            // ) -> Result<Vec<StakeUpdate>, std::io::Error> {
            //     let http_rpc_endpoint = #field_access.http_rpc_endpoint.clone();
            //     let contract_addresses = &gadget_sdk::config::ProtocolSpecificSettings::Eigenlayer.contract_addresses;
            //     eigensdk::client_avsregistry::reader::get_operator_stake_history(
            //         http_rpc_endpoint,
            //         contract_addresses,
            //         operator_id,
            //         quorum_number,
            //     ).await
            // }
            //
            // async fn get_operator_stake_update_at_index(
            //     &self,
            //     operator_id: FixedBytes<32>,
            //     quorum_number: u8,
            //     index: U256,
            // ) -> Result<StakeUpdate, std::io::Error> {
            //     let http_rpc_endpoint = #field_access.http_rpc_endpoint.clone();
            //     let contract_addresses = &gadget_sdk::config::ProtocolSpecificSettings::Eigenlayer.contract_addresses;
            //     eigensdk::client_avsregistry::reader::get_operator_stake_update_at_index(
            //         http_rpc_endpoint,
            //         contract_addresses,
            //         operator_id,
            //         quorum_number,
            //         index,
            //     ).await
            // }
            //
            // async fn get_operator_stake_at_block_number(
            //     &self,
            //     operator_id: FixedBytes<32>,
            //     quorum_number: u8,
            //     block_number: u32,
            // ) -> Result<U256, std::io::Error> {
            //     let http_rpc_endpoint = #field_access.http_rpc_endpoint.clone();
            //     let contract_addresses = &gadget_sdk::config::ProtocolSpecificSettings::Eigenlayer.contract_addresses;
            //     eigensdk::client_avsregistry::reader::get_operator_stake_at_block_number(
            //         http_rpc_endpoint,
            //         contract_addresses,
            //         operator_id,
            //         quorum_number,
            //         block_number,
            //     ).await
            // }
            //
            // async fn get_operator_details(
            //     &self,
            //     operator_addr: Address
            // ) -> Result<OperatorDetails, std::io::Error> {
            //     let http_rpc_endpoint = #field_access.http_rpc_endpoint.clone();
            //     let contract_addresses = &gadget_sdk::config::ProtocolSpecificSettings::Eigenlayer.contract_addresses;
            //     eigensdk::client_avsregistry::reader::get_operator_details(
            //         http_rpc_endpoint,
            //         contract_addresses,
            //         operator_addr,
            //     ).await
            // }
            //
            // async fn get_latest_stake_update(
            //     &self,
            //     operator_id: FixedBytes<32>,
            //     quorum_number: u8,
            // ) -> Result<StakeUpdate, std::io::Error> {
            //     let http_rpc_endpoint = #field_access.http_rpc_endpoint.clone();
            //     let contract_addresses = &gadget_sdk::config::ProtocolSpecificSettings::Eigenlayer.contract_addresses;
            //     eigensdk::client_avsregistry::reader::get_latest_stake_update(
            //         http_rpc_endpoint,
            //         contract_addresses,
            //         operator_id,
            //         quorum_number,
            //     ).await
            // }
            //
            // async fn get_operator_id(
            //     &self,
            //     operator_addr: Address,
            // ) -> Result<FixedBytes<32>, std::io::Error> {
            //     let http_rpc_endpoint = #field_access.http_rpc_endpoint.clone();
            //     let contract_addresses = &gadget_sdk::config::ProtocolSpecificSettings::Eigenlayer.contract_addresses;
            //     eigensdk::client_avsregistry::reader::get_operator_id(
            //         http_rpc_endpoint,
            //         contract_addresses,
            //         operator_addr,
            //     ).await
            // }
            //
            // async fn get_total_stake_at_block_number_from_index(
            //     &self,
            //     quorum_number: u8,
            //     block_number: u32,
            //     index: U256,
            // ) -> Result<U256, std::io::Error> {
            //     let http_rpc_endpoint = #field_access.http_rpc_endpoint.clone();
            //     let contract_addresses = &gadget_sdk::config::ProtocolSpecificSettings::Eigenlayer.contract_addresses;
            //     eigensdk::client_avsregistry::reader::get_total_stake_at_block_number_from_index(
            //         http_rpc_endpoint,
            //         contract_addresses,
            //         quorum_number,
            //         block_number,
            //         index,
            //     ).await
            // }
        }
    }
}
