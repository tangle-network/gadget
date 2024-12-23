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
        #[async_trait::async_trait]
        impl #impl_generics gadget_sdk::contexts::EigenlayerContext for #name #ty_generics #where_clause {
            async fn avs_registry_reader(&self) -> Result<eigensdk::client_avsregistry::reader::AvsRegistryChainReader, std::io::Error> {
                let http_rpc_endpoint = #field_access.http_rpc_endpoint.clone();
                let ::gadget_macros::config::ProtocolSpecificSettings::Eigenlayer(contract_addresses) = &#field_access.protocol_specific else {
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Expected Eigenlayer protocol"));
                };
                let registry_coordinator_address = contract_addresses.registry_coordinator_address;
                let operator_state_retriever_address = contract_addresses.operator_state_retriever_address;
                eigensdk::client_avsregistry::reader::AvsRegistryChainReader::new(
                    ::gadget_sdk::ext::eigensdk::logging::get_test_logger(),
                    registry_coordinator_address,
                    operator_state_retriever_address,
                    http_rpc_endpoint,
                ).await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
            }

            async fn avs_registry_writer(&self, private_key: String) -> Result<eigensdk::client_avsregistry::writer::AvsRegistryChainWriter, std::io::Error> {
                let http_rpc_endpoint = #field_access.http_rpc_endpoint.clone();
                let ::gadget_macros::config::ProtocolSpecificSettings::Eigenlayer(contract_addresses) = &#field_access.protocol_specific else {
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Expected Eigenlayer protocol"));
                };
                let registry_coordinator_address = contract_addresses.registry_coordinator_address;
                let operator_state_retriever_address = contract_addresses.operator_state_retriever_address;

                eigensdk::client_avsregistry::writer::AvsRegistryChainWriter::build_avs_registry_chain_writer(
                    ::gadget_sdk::ext::eigensdk::logging::get_test_logger(),
                    http_rpc_endpoint,
                    private_key,
                    registry_coordinator_address,
                    operator_state_retriever_address,
                ).await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
            }

            async fn operator_info_service_in_memory(&self) -> Result<
                (
                    ::gadget_sdk::ext::eigensdk::services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceInMemory,
                    ::gadget_sdk::tokio::sync::mpsc::UnboundedReceiver<::gadget_sdk::ext::eigensdk::services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceError>
                ),
                std::io::Error
            > {
                let avs_registry_reader = self.avs_registry_reader().await?;
                let ws_endpoint = #field_access.ws_rpc_endpoint.clone();

                eigensdk::services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceInMemory::new(
                    ::gadget_sdk::ext::eigensdk::logging::get_test_logger(),
                    avs_registry_reader,
                    ws_endpoint,
                ).await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
            }

            async fn avs_registry_service_chain_caller_in_memory(&self) -> Result<
                eigensdk::services_avsregistry::chaincaller::AvsRegistryServiceChainCaller<
                    eigensdk::client_avsregistry::reader::AvsRegistryChainReader,
                    eigensdk::services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceInMemory
                >, std::io::Error> {
                use ::gadget_sdk::ext::alloy_provider::Provider as _;

                let http_rpc_endpoint = #field_access.http_rpc_endpoint.clone();
                let avs_registry_reader = self.avs_registry_reader().await?;
                let (operator_info_service, _) = self.operator_info_service_in_memory().await?;

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
            ) -> Result<Vec<Vec<::gadget_sdk::ext::eigensdk::utils::operatorstateretriever::OperatorStateRetriever::Operator>>, std::io::Error> {
                use ::gadget_sdk::ext::eigensdk::client_avsregistry::reader::AvsRegistryReader as _;

                let http_rpc_endpoint = #field_access.http_rpc_endpoint.clone();
                self.avs_registry_reader().await?.get_operators_stake_in_quorums_at_block(
                    block_number,
                    quorum_numbers,
                ).await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
            }

            async fn get_operator_stake_in_quorums_at_current_block(
                &self,
                operator_id: ::gadget_sdk::ext::alloy_primitives::FixedBytes<32>,
            ) -> Result<HashMap<u8, ::gadget_sdk::contexts::BigInt>, std::io::Error> {
                let http_rpc_endpoint = #field_access.http_rpc_endpoint.clone();
                self.avs_registry_reader().await?.get_operator_stake_in_quorums_of_operator_at_current_block(
                    operator_id,
                ).await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
            }

            async fn get_operator_by_id(
                &self,
                operator_id: [u8; 32],
            ) -> Result<Address, std::io::Error> {
                use ::gadget_sdk::ext::eigensdk::client_avsregistry::reader::AvsRegistryReader as _;

                let http_rpc_endpoint = #field_access.http_rpc_endpoint.clone();
                self.avs_registry_reader().await?.get_operator_from_id(
                    operator_id,
                ).await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
            }

            async fn get_operator_stake_history(
                &self,
                operator_id: ::gadget_sdk::ext::alloy_primitives::FixedBytes<32>,
                quorum_number: u8,
            ) -> Result<Vec<::gadget_sdk::ext::eigensdk::utils::stakeregistry::IStakeRegistry::StakeUpdate>, std::io::Error> {
                use ::gadget_sdk::ext::alloy_provider::Provider as _;

                let http_rpc_endpoint = #field_access.http_rpc_endpoint.clone();
                let gadget_sdk::config::ProtocolSpecificSettings::Eigenlayer(contract_addresses) = &#field_access.protocol_specific else {
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Expected Eigenlayer protocol"));
                };
                let provider = alloy_provider::ProviderBuilder::new()
                    .with_recommended_fillers()
                    .on_http(http_rpc_endpoint.parse().map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?)
                    .root()
                    .clone()
                    .boxed();
                let registry_coordinator = ::gadget_sdk::ext::eigensdk::utils::registrycoordinator::RegistryCoordinator::new(contract_addresses.registry_coordinator_address, provider.clone());
                let stake_registry_address = registry_coordinator.stakeRegistry().call().await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?._0;
                let instance = ::gadget_sdk::ext::eigensdk::utils::stakeregistry::StakeRegistry::StakeRegistryInstance::new(stake_registry_address, provider.clone());
                let call_builder = instance.getStakeHistory(operator_id, quorum_number);
                let response = call_builder.call().await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                Ok(response._0)
            }

            async fn get_operator_stake_update_at_index(
                &self,
                quorum_number: u8,
                operator_id: ::gadget_sdk::ext::alloy_primitives::FixedBytes<32>,
                index: ::gadget_sdk::ext::alloy_primitives::U256,
            ) -> Result<::gadget_sdk::ext::eigensdk::utils::stakeregistry::IStakeRegistry::StakeUpdate, std::io::Error> {
                use ::gadget_sdk::ext::alloy_provider::Provider as _;

                let http_rpc_endpoint = #field_access.http_rpc_endpoint.clone();
                let gadget_sdk::config::ProtocolSpecificSettings::Eigenlayer(contract_addresses) = &#field_access.protocol_specific else {
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Expected Eigenlayer protocol"));
                };
                let provider = alloy_provider::ProviderBuilder::new()
                    .with_recommended_fillers()
                    .on_http(http_rpc_endpoint.parse().map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?)
                    .root()
                    .clone()
                    .boxed();
                let registry_coordinator = ::gadget_sdk::ext::eigensdk::utils::registrycoordinator::RegistryCoordinator::new(contract_addresses.registry_coordinator_address, provider.clone());
                let stake_registry_address = registry_coordinator.stakeRegistry().call().await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?._0;
                let instance = ::gadget_sdk::ext::eigensdk::utils::stakeregistry::StakeRegistry::StakeRegistryInstance::new(stake_registry_address, provider.clone());
                let call_builder = instance.getStakeUpdateAtIndex(quorum_number, operator_id, index);
                let response = call_builder.call().await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                Ok(response._0)
            }

            async fn get_operator_stake_at_block_number(
                &self,
                operator_id: ::gadget_sdk::ext::alloy_primitives::FixedBytes<32>,
                quorum_number: u8,
                block_number: u32,
            ) -> Result<::gadget_sdk::ext::alloy_primitives::Uint<96, 2>, std::io::Error> {
                use ::gadget_sdk::ext::alloy_provider::Provider as _;

                let http_rpc_endpoint = #field_access.http_rpc_endpoint.clone();
                let gadget_sdk::config::ProtocolSpecificSettings::Eigenlayer(contract_addresses) = &#field_access.protocol_specific else {
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Expected Eigenlayer protocol"));
                };
                let provider = alloy_provider::ProviderBuilder::new()
                    .with_recommended_fillers()
                    .on_http(http_rpc_endpoint.parse().map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?)
                    .root()
                    .clone()
                    .boxed();
                let registry_coordinator = ::gadget_sdk::ext::eigensdk::utils::registrycoordinator::RegistryCoordinator::new(contract_addresses.registry_coordinator_address, provider.clone());
                let stake_registry_address = registry_coordinator.stakeRegistry().call().await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?._0;
                let instance = ::gadget_sdk::ext::eigensdk::utils::stakeregistry::StakeRegistry::StakeRegistryInstance::new(stake_registry_address, provider.clone());
                let call_builder = instance.getStakeAtBlockNumber(operator_id, quorum_number, block_number);
                let response = call_builder.call().await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                Ok(response._0)
            }

            async fn get_operator_details(
                &self,
                operator_addr: Address
            ) -> Result<::gadget_sdk::ext::eigensdk::types::operator::Operator, std::io::Error> {
                use ::gadget_sdk::ext::alloy_provider::Provider as _;

                let http_rpc_endpoint = #field_access.http_rpc_endpoint.clone();
                let gadget_sdk::config::ProtocolSpecificSettings::Eigenlayer(contract_addresses) = &#field_access.protocol_specific else {
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Expected Eigenlayer protocol"));
                };
                let provider = alloy_provider::ProviderBuilder::new()
                    .with_recommended_fillers()
                    .on_http(http_rpc_endpoint.parse().map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?)
                    .root()
                    .clone()
                    .boxed();
                let slasher_addr = ::gadget_sdk::utils::evm::get_slasher_address(contract_addresses.delegation_manager_address, &http_rpc_endpoint.clone()).await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                let chain_reader = ::gadget_sdk::ext::eigensdk::client_elcontracts::reader::ELChainReader::new(
                    ::gadget_sdk::ext::eigensdk::logging::get_test_logger(),
                    slasher_addr,
                    contract_addresses.delegation_manager_address,
                    contract_addresses.avs_directory_address,
                    http_rpc_endpoint.clone(),
                );
                Ok(chain_reader.get_operator_details(operator_addr).await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?)

            }

            async fn get_latest_stake_update(
                &self,
                operator_id: ::gadget_sdk::ext::alloy_primitives::FixedBytes<32>,
                quorum_number: u8,
            ) -> Result<::gadget_sdk::ext::eigensdk::utils::stakeregistry::IStakeRegistry::StakeUpdate, std::io::Error> {
                use ::gadget_sdk::ext::alloy_provider::Provider as _;

                let http_rpc_endpoint = #field_access.http_rpc_endpoint.clone();
                let gadget_sdk::config::ProtocolSpecificSettings::Eigenlayer(contract_addresses) = &#field_access.protocol_specific else {
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Expected Eigenlayer protocol"));
                };
                let provider = alloy_provider::ProviderBuilder::new()
                    .with_recommended_fillers()
                    .on_http(http_rpc_endpoint.parse().map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?)
                    .root()
                    .clone()
                    .boxed();
                let registry_coordinator = ::gadget_sdk::ext::eigensdk::utils::registrycoordinator::RegistryCoordinator::new(contract_addresses.registry_coordinator_address, provider.clone());
                let stake_registry_address = registry_coordinator.stakeRegistry().call().await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?._0;
                let instance = ::gadget_sdk::ext::eigensdk::utils::stakeregistry::StakeRegistry::StakeRegistryInstance::new(stake_registry_address, provider.clone());
                let call_builder = instance.getLatestStakeUpdate(operator_id, quorum_number);
                let response = call_builder.call().await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                Ok(response._0)
            }

            async fn get_operator_id(
                &self,
                operator_addr: Address,
            ) -> Result<::gadget_sdk::ext::alloy_primitives::FixedBytes<32>, std::io::Error> {
                let http_rpc_endpoint = #field_access.http_rpc_endpoint.clone();
                self.avs_registry_reader().await?.get_operator_id(
                    operator_addr,
                ).await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
            }

            async fn get_total_stake_at_block_number_from_index(
                &self,
                quorum_number: u8,
                block_number: u32,
                index: ::gadget_sdk::ext::alloy_primitives::U256,
            ) -> Result<alloy_primitives::Uint<96, 2>, std::io::Error> {
                use ::gadget_sdk::ext::alloy_provider::Provider as _;

                let http_rpc_endpoint = #field_access.http_rpc_endpoint.clone();
                let gadget_sdk::config::ProtocolSpecificSettings::Eigenlayer(contract_addresses) = &#field_access.protocol_specific else {
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Expected Eigenlayer protocol"));
                };
                let provider = alloy_provider::ProviderBuilder::new()
                    .with_recommended_fillers()
                    .on_http(http_rpc_endpoint.parse().map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?)
                    .root()
                    .clone()
                    .boxed();
                let registry_coordinator = ::gadget_sdk::ext::eigensdk::utils::registrycoordinator::RegistryCoordinator::new(contract_addresses.registry_coordinator_address, provider.clone());
                let stake_registry_address = registry_coordinator.stakeRegistry().call().await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?._0;
                let instance = ::gadget_sdk::ext::eigensdk::utils::stakeregistry::StakeRegistry::StakeRegistryInstance::new(stake_registry_address, provider.clone());
                let call_builder = instance.getTotalStakeAtBlockNumberFromIndex(quorum_number, block_number, index);
                let response = call_builder.call().await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                Ok(response._0)
            }

            async fn get_total_stake_history_length(
                &self,
                quorum_number: u8,
            ) -> Result<::gadget_sdk::ext::alloy_primitives::U256, std::io::Error> {
                use ::gadget_sdk::ext::alloy_provider::Provider as _;

                let http_rpc_endpoint = #field_access.http_rpc_endpoint.clone();
                let gadget_sdk::config::ProtocolSpecificSettings::Eigenlayer(contract_addresses) = &#field_access.protocol_specific else {
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Expected Eigenlayer protocol"));
                };
                let provider = alloy_provider::ProviderBuilder::new()
                    .with_recommended_fillers()
                    .on_http(http_rpc_endpoint.parse().map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?)
                    .root()
                    .clone()
                    .boxed();
                let registry_coordinator = ::gadget_sdk::ext::eigensdk::utils::registrycoordinator::RegistryCoordinator::new(contract_addresses.registry_coordinator_address, provider.clone());
                let stake_registry_address = registry_coordinator.stakeRegistry().call().await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?._0;
                let instance = ::gadget_sdk::ext::eigensdk::utils::stakeregistry::StakeRegistry::StakeRegistryInstance::new(stake_registry_address, provider.clone());
                let call_builder = instance.getTotalStakeHistoryLength(quorum_number);
                let response = call_builder.call().await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                Ok(response._0)
            }

            async fn query_existing_registered_operator_pub_keys(
                &self,
                start_block: u64,
                to_block: u64,
            ) -> Result<(Vec<Address>, Vec<::gadget_sdk::ext::eigensdk::types::operator::OperatorPubKeys>), std::io::Error> {
                let ws_rpc_endpoint = #field_access.ws_rpc_endpoint.clone();
                self.avs_registry_reader().await?.query_existing_registered_operator_pub_keys(
                    start_block,
                    to_block,
                    ws_rpc_endpoint,
                ).await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
            }
        }
    }
}
