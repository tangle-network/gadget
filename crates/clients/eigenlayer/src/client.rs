use crate::error::Result;
use alloy_primitives::{Address, Bytes};
use alloy_provider::{Provider, RootProvider};
use alloy_pubsub::PubSubFrontend;
use alloy_transport::BoxTransport;
use eigensdk::client_avsregistry::reader::AvsRegistryReader;
use eigensdk::common::get_ws_provider;
use eigensdk::logging::get_test_logger;
use gadget_config::GadgetConfiguration;
use gadget_std::collections::HashMap;
use gadget_utils_evm::{get_provider_http, get_wallet_provider_http};
use num_bigint::BigInt;

/// Client that provides access to EigenLayer utility functions through the use of the [`GadgetConfiguration`].
#[derive(Clone)]
pub struct EigenlayerClient {
    pub config: GadgetConfiguration,
}

impl EigenlayerClient {
    /// Creates a new instance of the [`EigenlayerClient`] given a [`GadgetConfiguration`].
    pub fn new(config: GadgetConfiguration) -> Self {
        Self { config }
    }

    /// Get the [`GadgetConfiguration`] for this client
    pub fn config(&self) -> &GadgetConfiguration {
        &self.config
    }

    /// Get the provider for this client's http endpoint
    ///
    /// # Returns
    /// - [`The HTTP provider`](RootProvider<BoxTransport>)
    pub fn get_provider_http(&self) -> RootProvider<BoxTransport> {
        get_provider_http(&self.config.http_rpc_endpoint)
    }

    /// Get the provider for this client's http endpoint with the specified [`Wallet`](EthereumWallet)
    ///
    /// # Returns
    /// - [`The HTTP wallet provider`](RootProvider<BoxTransport>)
    pub fn get_wallet_provider_http(
        &self,
        wallet: alloy_network::EthereumWallet,
    ) -> RootProvider<BoxTransport> {
        get_wallet_provider_http(&self.config.http_rpc_endpoint, wallet)
    }

    /// Get the provider for this client's websocket endpoint
    ///
    /// # Returns
    /// - [`The WS provider`](RootProvider<BoxTransport>)
    pub async fn get_provider_ws(&self) -> Result<RootProvider<PubSubFrontend>> {
        get_ws_provider(&self.config.ws_rpc_endpoint)
            .await
            .map_err(Into::into)
    }

    /// Get the slasher address from the `DelegationManager` contract
    ///
    /// # Returns
    /// - [`Address`] - The slasher address
    ///
    /// # Errors
    /// - [`Error::AlloyContract`] - If the call to the contract fails (i.e. the contract doesn't exist at the given address)
    pub async fn get_slasher_address(&self, delegation_manager_addr: Address) -> Result<Address> {
        let provider = self.get_provider_http();
        let delegation_manager =
            eigensdk::utils::core::delegationmanager::DelegationManager::DelegationManagerInstance::new(
                delegation_manager_addr,
                provider,
            );
        delegation_manager
            .slasher()
            .call()
            .await
            .map(|a| a._0)
            .map_err(Into::into)
    }

    /// Provides a reader for the AVS registry.
    pub async fn avs_registry_reader(
        &self,
    ) -> Result<eigensdk::client_avsregistry::reader::AvsRegistryChainReader> {
        let http_rpc_endpoint = self.config.http_rpc_endpoint.clone();
        let contract_addresses = self.config.protocol_settings.eigenlayer()?;
        let registry_coordinator_address = contract_addresses.registry_coordinator_address;
        let operator_state_retriever_address = contract_addresses.operator_state_retriever_address;
        eigensdk::client_avsregistry::reader::AvsRegistryChainReader::new(
            eigensdk::logging::get_test_logger(),
            registry_coordinator_address,
            operator_state_retriever_address,
            http_rpc_endpoint,
        )
        .await
        .map_err(Into::into)
    }

    /// Provides a writer for the AVS registry.
    pub async fn avs_registry_writer(
        &self,
        private_key: String,
    ) -> Result<eigensdk::client_avsregistry::writer::AvsRegistryChainWriter> {
        let http_rpc_endpoint = self.config.http_rpc_endpoint.clone();
        let contract_addresses = self.config.protocol_settings.eigenlayer()?;
        let registry_coordinator_address = contract_addresses.registry_coordinator_address;
        let operator_state_retriever_address = contract_addresses.operator_state_retriever_address;

        eigensdk::client_avsregistry::writer::AvsRegistryChainWriter::build_avs_registry_chain_writer(
            eigensdk::logging::get_test_logger(),
            http_rpc_endpoint,
            private_key,
            registry_coordinator_address,
            operator_state_retriever_address,
        ).await
        .map_err(Into::into)
    }

    /// Provides an operator info service.
    pub async fn operator_info_service_in_memory(
        &self,
    ) -> Result<(
        eigensdk::services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceInMemory,
        tokio::sync::mpsc::UnboundedReceiver<
            eigensdk::services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceError,
        >,
    )> {
        let avs_registry_reader = self.avs_registry_reader().await?;
        let ws_endpoint = self.config.ws_rpc_endpoint.clone();

        eigensdk::services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceInMemory::new(
            eigensdk::logging::get_test_logger(),
            avs_registry_reader,
            ws_endpoint,
        )
        .await
        .map_err(Into::into)
    }

    /// Provides an AVS registry service chain caller.
    pub async fn avs_registry_service_chain_caller_in_memory(
        &self,
    ) -> Result<
        eigensdk::services_avsregistry::chaincaller::AvsRegistryServiceChainCaller<
            eigensdk::client_avsregistry::reader::AvsRegistryChainReader,
            eigensdk::services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceInMemory,
        >,
    > {
        let avs_registry_reader = self.avs_registry_reader().await?;
        let (operator_info_service, _) = self.operator_info_service_in_memory().await?;

        let cancellation_token = tokio_util::sync::CancellationToken::new();
        let token_clone = cancellation_token.clone();
        let provider = self.get_provider_http();
        let current_block = provider.get_block_number().await?;
        let operator_info_clone = operator_info_service.clone();

        tokio::task::spawn(async move {
            operator_info_clone
                .start_service(&token_clone, 0, current_block)
                .await
        });

        Ok(
            eigensdk::services_avsregistry::chaincaller::AvsRegistryServiceChainCaller::new(
                avs_registry_reader,
                operator_info_service,
            ),
        )
    }

    /// Provides a BLS aggregation service.
    pub async fn bls_aggregation_service_in_memory(&self) -> Result<eigensdk::services_blsaggregation::bls_agg::BlsAggregatorService<
        eigensdk::services_avsregistry::chaincaller::AvsRegistryServiceChainCaller<
            eigensdk::client_avsregistry::reader::AvsRegistryChainReader,
            eigensdk::services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceInMemory
        >
    >>{
        let avs_registry_service = self.avs_registry_service_chain_caller_in_memory().await?;
        Ok(
            eigensdk::services_blsaggregation::bls_agg::BlsAggregatorService::new(
                avs_registry_service,
                get_test_logger(),
            ),
        )
    }

    /// Get Operator stake in Quorums at a given block.
    pub async fn get_operator_stake_in_quorums_at_block(
        &self,
        block_number: u32,
        quorum_numbers: Bytes,
    ) -> Result<Vec<Vec<eigensdk::utils::middleware::operatorstateretriever::OperatorStateRetriever::Operator>>>
    {
        self.avs_registry_reader()
            .await?
            .get_operators_stake_in_quorums_at_block(block_number, quorum_numbers)
            .await
            .map_err(Into::into)
    }

    /// Get an Operator's stake in Quorums at current block.
    pub async fn get_operator_stake_in_quorums_at_current_block(
        &self,
        operator_id: alloy_primitives::FixedBytes<32>,
    ) -> Result<HashMap<u8, BigInt>> {
        self.avs_registry_reader()
            .await?
            .get_operator_stake_in_quorums_of_operator_at_current_block(operator_id)
            .await
            .map_err(Into::into)
    }

    /// Get an Operator by ID.
    pub async fn get_operator_by_id(&self, operator_id: [u8; 32]) -> Result<Address> {
        self.avs_registry_reader()
            .await?
            .get_operator_from_id(operator_id)
            .await
            .map_err(Into::into)
    }

    /// Get an Operator stake history.
    pub async fn get_operator_stake_history(
        &self,
        operator_id: alloy_primitives::FixedBytes<32>,
        quorum_number: u8,
    ) -> Result<Vec<eigensdk::utils::middleware::stakeregistry::IStakeRegistry::StakeUpdate>> {
        let contract_addresses = self.config.protocol_settings.eigenlayer()?;
        let provider = self.get_provider_http();
        let registry_coordinator =
            eigensdk::utils::middleware::registrycoordinator::RegistryCoordinator::new(
                contract_addresses.registry_coordinator_address,
                provider.clone(),
            );
        let stake_registry_address = registry_coordinator.stakeRegistry().call().await?._0;
        let instance =
            eigensdk::utils::middleware::stakeregistry::StakeRegistry::StakeRegistryInstance::new(
                stake_registry_address,
                provider.clone(),
            );
        let call_builder = instance.getStakeHistory(operator_id, quorum_number);
        let response = call_builder.call().await?;
        Ok(response._0)
    }

    /// Get an Operator stake update at a given index.
    pub async fn get_operator_stake_update_at_index(
        &self,
        quorum_number: u8,
        operator_id: alloy_primitives::FixedBytes<32>,
        index: alloy_primitives::U256,
    ) -> Result<eigensdk::utils::middleware::stakeregistry::IStakeRegistry::StakeUpdate> {
        let contract_addresses = self.config.protocol_settings.eigenlayer()?;
        let provider = self.get_provider_http();
        let registry_coordinator =
            eigensdk::utils::middleware::registrycoordinator::RegistryCoordinator::new(
                contract_addresses.registry_coordinator_address,
                provider.clone(),
            );
        let stake_registry_address = registry_coordinator.stakeRegistry().call().await?._0;
        let instance =
            eigensdk::utils::middleware::stakeregistry::StakeRegistry::StakeRegistryInstance::new(
                stake_registry_address,
                provider.clone(),
            );
        let call_builder = instance.getStakeUpdateAtIndex(quorum_number, operator_id, index);
        let response = call_builder.call().await?;
        Ok(response._0)
    }

    /// Get an Operator's stake at a given block number.
    pub async fn get_operator_stake_at_block_number(
        &self,
        operator_id: alloy_primitives::FixedBytes<32>,
        quorum_number: u8,
        block_number: u32,
    ) -> Result<alloy_primitives::Uint<96, 2>> {
        let contract_addresses = self.config.protocol_settings.eigenlayer()?;
        let provider = self.get_provider_http();
        let registry_coordinator =
            eigensdk::utils::middleware::registrycoordinator::RegistryCoordinator::new(
                contract_addresses.registry_coordinator_address,
                provider.clone(),
            );
        let stake_registry_address = registry_coordinator.stakeRegistry().call().await?._0;
        let instance =
            eigensdk::utils::middleware::stakeregistry::StakeRegistry::StakeRegistryInstance::new(
                stake_registry_address,
                provider.clone(),
            );
        let call_builder = instance.getStakeAtBlockNumber(operator_id, quorum_number, block_number);
        let response = call_builder.call().await?;
        Ok(response._0)
    }

    /// Get an Operator's [`details`](OperatorDetails).
    pub async fn get_operator_details(
        &self,
        operator_addr: Address,
    ) -> Result<eigensdk::types::operator::Operator> {
        let http_rpc_endpoint = self.config.http_rpc_endpoint.clone();
        let contract_addresses = self.config.protocol_settings.eigenlayer()?;
        let slasher_addr = self
            .get_slasher_address(contract_addresses.delegation_manager_address)
            .await?;
        let chain_reader = eigensdk::client_elcontracts::reader::ELChainReader::new(
            eigensdk::logging::get_test_logger(),
            slasher_addr,
            contract_addresses.delegation_manager_address,
            contract_addresses.rewards_coordinator_address,
            contract_addresses.avs_directory_address,
            http_rpc_endpoint.clone(),
        );
        Ok(chain_reader.get_operator_details(operator_addr).await?)
    }

    /// Get an Operator's latest stake update.
    pub async fn get_latest_stake_update(
        &self,
        operator_id: alloy_primitives::FixedBytes<32>,
        quorum_number: u8,
    ) -> Result<eigensdk::utils::middleware::stakeregistry::IStakeRegistry::StakeUpdate> {
        let contract_addresses = self.config.protocol_settings.eigenlayer()?;
        let provider = self.get_provider_http();
        let registry_coordinator =
            eigensdk::utils::middleware::registrycoordinator::RegistryCoordinator::new(
                contract_addresses.registry_coordinator_address,
                provider.clone(),
            );
        let stake_registry_address = registry_coordinator.stakeRegistry().call().await?._0;
        let instance =
            eigensdk::utils::middleware::stakeregistry::StakeRegistry::StakeRegistryInstance::new(
                stake_registry_address,
                provider.clone(),
            );
        let call_builder = instance.getLatestStakeUpdate(operator_id, quorum_number);
        let response = call_builder.call().await?;
        Ok(response._0)
    }

    /// Get an Operator's ID as [`FixedBytes`] from its [`Address`].
    pub async fn get_operator_id(
        &self,
        operator_addr: Address,
    ) -> Result<alloy_primitives::FixedBytes<32>> {
        self.avs_registry_reader()
            .await?
            .get_operator_id(operator_addr)
            .await
            .map_err(Into::into)
    }

    /// Get the total stake at a given block number from a given index.
    pub async fn get_total_stake_at_block_number_from_index(
        &self,
        quorum_number: u8,
        block_number: u32,
        index: alloy_primitives::U256,
    ) -> Result<alloy_primitives::Uint<96, 2>> {
        let contract_addresses = self.config.protocol_settings.eigenlayer()?;
        let provider = self.get_provider_http();
        let registry_coordinator =
            eigensdk::utils::middleware::registrycoordinator::RegistryCoordinator::new(
                contract_addresses.registry_coordinator_address,
                provider.clone(),
            );
        let stake_registry_address = registry_coordinator.stakeRegistry().call().await?._0;
        let instance =
            eigensdk::utils::middleware::stakeregistry::StakeRegistry::StakeRegistryInstance::new(
                stake_registry_address,
                provider.clone(),
            );
        let call_builder =
            instance.getTotalStakeAtBlockNumberFromIndex(quorum_number, block_number, index);
        let response = call_builder.call().await?;
        Ok(response._0)
    }

    /// Get the total stake history length of a given quorum.
    pub async fn get_total_stake_history_length(
        &self,
        quorum_number: u8,
    ) -> Result<alloy_primitives::U256> {
        let contract_addresses = self.config.protocol_settings.eigenlayer()?;
        let provider = self.get_provider_http();
        let registry_coordinator =
            eigensdk::utils::middleware::registrycoordinator::RegistryCoordinator::new(
                contract_addresses.registry_coordinator_address,
                provider.clone(),
            );
        let stake_registry_address = registry_coordinator.stakeRegistry().call().await?._0;
        let instance =
            eigensdk::utils::middleware::stakeregistry::StakeRegistry::StakeRegistryInstance::new(
                stake_registry_address,
                provider.clone(),
            );
        let call_builder = instance.getTotalStakeHistoryLength(quorum_number);
        let response = call_builder.call().await?;
        Ok(response._0)
    }

    /// Provides the public keys of existing registered operators within the provided block range.
    pub async fn query_existing_registered_operator_pub_keys(
        &self,
        start_block: u64,
        to_block: u64,
    ) -> Result<(
        Vec<Address>,
        Vec<eigensdk::types::operator::OperatorPubKeys>,
    )> {
        let ws_rpc_endpoint = self.config.ws_rpc_endpoint.clone();
        self.avs_registry_reader()
            .await?
            .query_existing_registered_operator_pub_keys(start_block, to_block, ws_rpc_endpoint)
            .await
            .map_err(Into::into)
    }
}
