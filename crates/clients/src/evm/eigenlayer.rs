use alloy_primitives::{Address, Bytes};
use alloy_provider::{Provider, ProviderBuilder, RootProvider};
use alloy_transport::BoxTransport;
use eigensdk::client_avsregistry::reader::AvsRegistryReader;
use gadget_config::GadgetConfiguration;
use num_bigint::BigInt;
use std::collections::HashMap;

/// Client that provides access to EigenLayer utility functions through the use of the [`GadgetConfiguration`].
pub struct EigenlayerClient {
    pub config: GadgetConfiguration,
}

impl EigenlayerClient {
    /// Get the provider for this client's http endpoint
    ///
    /// # Returns
    /// - [`The HTTP provider`](RootProvider<BoxTransport>)
    pub fn get_provider_http(&self) -> Result<RootProvider<BoxTransport>, std::io::Error> {
        let http_endpoint = self.config.http_rpc_endpoint.clone();
        let http_endpoint = http_endpoint
            .parse()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .on_http(http_endpoint)
            .root()
            .clone()
            .boxed();
        Ok(provider)
    }

    /// Get the provider for this client's http endpoint with the specified [`Wallet`](EthereumWallet)
    ///
    /// # Returns
    /// - [`The HTTP wallet provider`](RootProvider<BoxTransport>)
    pub fn get_wallet_provider_http(
        &self,
        wallet: alloy_network::EthereumWallet,
    ) -> Result<RootProvider<BoxTransport>, std::io::Error> {
        let http_endpoint = self.config.http_rpc_endpoint.clone();
        let http_endpoint = http_endpoint
            .parse()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_http(http_endpoint)
            .root()
            .clone()
            .boxed();
        Ok(provider)
    }

    /// Get the provider for this client's websocket endpoint
    ///
    /// # Returns
    /// - [`The WS provider`](RootProvider<BoxTransport>)
    pub async fn get_provider_ws(&self) -> Result<RootProvider<BoxTransport>, std::io::Error> {
        let ws_endpoint = self.config.ws_rpc_endpoint.clone();
        let ws_endpoint = ws_endpoint
            .parse()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .on_ws(alloy_provider::WsConnect::new(ws_endpoint))
            .await
            .unwrap()
            .root()
            .clone()
            .boxed();
        Ok(provider)
    }

    /// Get the slasher address from the `DelegationManager` contract
    ///
    /// # Returns
    /// - [`Address`] - The slasher address
    ///
    /// # Errors
    /// - [`Error::AlloyContract`] - If the call to the contract fails (i.e. the contract doesn't exist at the given address)
    pub async fn get_slasher_address(
        &self,
        delegation_manager_addr: Address,
    ) -> Result<Address, std::io::Error> {
        let provider = self.get_provider_http()?;
        let delegation_manager =
            eigensdk::utils::delegationmanager::DelegationManager::DelegationManagerInstance::new(
                delegation_manager_addr,
                provider,
            );
        delegation_manager
            .slasher()
            .call()
            .await
            .map(|a| a._0)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    /// Provides a reader for the AVS registry.
    async fn avs_registry_reader(
        &self,
    ) -> Result<eigensdk::client_avsregistry::reader::AvsRegistryChainReader, std::io::Error> {
        let http_rpc_endpoint = self.config.http_rpc_endpoint.clone();
        let contract_addresses = self
            .config
            .protocol_settings
            .eigenlayer()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let registry_coordinator_address = contract_addresses.registry_coordinator_address;
        let operator_state_retriever_address = contract_addresses.operator_state_retriever_address;
        eigensdk::client_avsregistry::reader::AvsRegistryChainReader::new(
            eigensdk::logging::get_test_logger(),
            registry_coordinator_address,
            operator_state_retriever_address,
            http_rpc_endpoint,
        )
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    /// Provides a writer for the AVS registry.
    async fn avs_registry_writer(
        &self,
        private_key: String,
    ) -> Result<eigensdk::client_avsregistry::writer::AvsRegistryChainWriter, std::io::Error> {
        let http_rpc_endpoint = self.config.http_rpc_endpoint.clone();
        let contract_addresses = self
            .config
            .protocol_settings
            .eigenlayer()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
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

    /// Provides an operator info service.
    async fn operator_info_service_in_memory(
        &self,
    ) -> Result<
        (
            eigensdk::services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceInMemory,
            tokio::sync::mpsc::UnboundedReceiver<
                eigensdk::services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceError,
            >,
        ),
        std::io::Error,
    > {
        let avs_registry_reader = self.avs_registry_reader().await?;
        let ws_endpoint = self.config.ws_rpc_endpoint.clone();

        eigensdk::services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceInMemory::new(
            eigensdk::logging::get_test_logger(),
            avs_registry_reader,
            ws_endpoint,
        )
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    /// Provides an AVS registry service chain caller.
    async fn avs_registry_service_chain_caller_in_memory(
        &self,
    ) -> Result<
        eigensdk::services_avsregistry::chaincaller::AvsRegistryServiceChainCaller<
            eigensdk::client_avsregistry::reader::AvsRegistryChainReader,
            eigensdk::services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceInMemory,
        >,
        std::io::Error,
    > {
        let avs_registry_reader = self.avs_registry_reader().await?;
        let (operator_info_service, _) = self.operator_info_service_in_memory().await?;

        let cancellation_token = tokio_util::sync::CancellationToken::new();
        let token_clone = cancellation_token.clone();
        let provider = self.get_provider_http()?;
        let current_block = provider
            .get_block_number()
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
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
    async fn bls_aggregation_service_in_memory(&self) -> Result<eigensdk::services_blsaggregation::bls_agg::BlsAggregatorService<
        eigensdk::services_avsregistry::chaincaller::AvsRegistryServiceChainCaller<
            eigensdk::client_avsregistry::reader::AvsRegistryChainReader,
            eigensdk::services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceInMemory
        >
    >, std::io::Error>{
        let avs_registry_service = self.avs_registry_service_chain_caller_in_memory().await?;
        Ok(
            eigensdk::services_blsaggregation::bls_agg::BlsAggregatorService::new(
                avs_registry_service,
            ),
        )
    }

    /// Get Operator stake in Quorums at a given block.
    async fn get_operator_stake_in_quorums_at_block(
        &self,
        block_number: u32,
        quorum_numbers: Bytes,
    ) -> Result<
        Vec<Vec<eigensdk::utils::operatorstateretriever::OperatorStateRetriever::Operator>>,
        std::io::Error,
    > {
        self.avs_registry_reader()
            .await?
            .get_operators_stake_in_quorums_at_block(block_number, quorum_numbers)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    /// Get an Operator's stake in Quorums at current block.
    async fn get_operator_stake_in_quorums_at_current_block(
        &self,
        operator_id: alloy_primitives::FixedBytes<32>,
    ) -> Result<HashMap<u8, BigInt>, std::io::Error> {
        self.avs_registry_reader()
            .await?
            .get_operator_stake_in_quorums_of_operator_at_current_block(operator_id)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    /// Get an Operator by ID.
    async fn get_operator_by_id(&self, operator_id: [u8; 32]) -> Result<Address, std::io::Error> {
        self.avs_registry_reader()
            .await?
            .get_operator_from_id(operator_id)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    /// Get an Operator stake history.
    async fn get_operator_stake_history(
        &self,
        operator_id: alloy_primitives::FixedBytes<32>,
        quorum_number: u8,
    ) -> Result<Vec<eigensdk::utils::stakeregistry::IStakeRegistry::StakeUpdate>, std::io::Error>
    {
        let contract_addresses = self
            .config
            .protocol_settings
            .eigenlayer()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let provider = self.get_provider_http()?;
        let registry_coordinator = eigensdk::utils::registrycoordinator::RegistryCoordinator::new(
            contract_addresses.registry_coordinator_address,
            provider.clone(),
        );
        let stake_registry_address = registry_coordinator
            .stakeRegistry()
            .call()
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
            ._0;
        let instance = eigensdk::utils::stakeregistry::StakeRegistry::StakeRegistryInstance::new(
            stake_registry_address,
            provider.clone(),
        );
        let call_builder = instance.getStakeHistory(operator_id, quorum_number);
        let response = call_builder
            .call()
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(response._0)
    }

    /// Get an Operator stake update at a given index.
    async fn get_operator_stake_update_at_index(
        &self,
        quorum_number: u8,
        operator_id: alloy_primitives::FixedBytes<32>,
        index: alloy_primitives::U256,
    ) -> Result<eigensdk::utils::stakeregistry::IStakeRegistry::StakeUpdate, std::io::Error> {
        let contract_addresses = self
            .config
            .protocol_settings
            .eigenlayer()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let provider = self.get_provider_http()?;
        let registry_coordinator = eigensdk::utils::registrycoordinator::RegistryCoordinator::new(
            contract_addresses.registry_coordinator_address,
            provider.clone(),
        );
        let stake_registry_address = registry_coordinator
            .stakeRegistry()
            .call()
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
            ._0;
        let instance = eigensdk::utils::stakeregistry::StakeRegistry::StakeRegistryInstance::new(
            stake_registry_address,
            provider.clone(),
        );
        let call_builder = instance.getStakeUpdateAtIndex(quorum_number, operator_id, index);
        let response = call_builder
            .call()
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(response._0)
    }

    /// Get an Operator's stake at a given block number.
    async fn get_operator_stake_at_block_number(
        &self,
        operator_id: alloy_primitives::FixedBytes<32>,
        quorum_number: u8,
        block_number: u32,
    ) -> Result<alloy_primitives::Uint<96, 2>, std::io::Error> {
        let contract_addresses = self
            .config
            .protocol_settings
            .eigenlayer()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let provider = self.get_provider_http()?;
        let registry_coordinator = eigensdk::utils::registrycoordinator::RegistryCoordinator::new(
            contract_addresses.registry_coordinator_address,
            provider.clone(),
        );
        let stake_registry_address = registry_coordinator
            .stakeRegistry()
            .call()
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
            ._0;
        let instance = eigensdk::utils::stakeregistry::StakeRegistry::StakeRegistryInstance::new(
            stake_registry_address,
            provider.clone(),
        );
        let call_builder = instance.getStakeAtBlockNumber(operator_id, quorum_number, block_number);
        let response = call_builder
            .call()
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(response._0)
    }

    /// Get an Operator's [`details`](OperatorDetails).
    async fn get_operator_details(
        &self,
        operator_addr: Address,
    ) -> Result<eigensdk::types::operator::Operator, std::io::Error> {
        let http_rpc_endpoint = self.config.http_rpc_endpoint.clone();
        let contract_addresses = self
            .config
            .protocol_settings
            .eigenlayer()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let slasher_addr = self
            .get_slasher_address(contract_addresses.delegation_manager_address)
            .await?;
        let chain_reader = eigensdk::client_elcontracts::reader::ELChainReader::new(
            eigensdk::logging::get_test_logger(),
            slasher_addr,
            contract_addresses.delegation_manager_address,
            contract_addresses.avs_directory_address,
            http_rpc_endpoint.clone(),
        );
        Ok(chain_reader
            .get_operator_details(operator_addr)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?)
    }

    /// Get an Operator's latest stake update.
    async fn get_latest_stake_update(
        &self,
        operator_id: alloy_primitives::FixedBytes<32>,
        quorum_number: u8,
    ) -> Result<eigensdk::utils::stakeregistry::IStakeRegistry::StakeUpdate, std::io::Error> {
        let contract_addresses = self
            .config
            .protocol_settings
            .eigenlayer()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let provider = self.get_provider_http()?;
        let registry_coordinator = eigensdk::utils::registrycoordinator::RegistryCoordinator::new(
            contract_addresses.registry_coordinator_address,
            provider.clone(),
        );
        let stake_registry_address = registry_coordinator
            .stakeRegistry()
            .call()
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
            ._0;
        let instance = eigensdk::utils::stakeregistry::StakeRegistry::StakeRegistryInstance::new(
            stake_registry_address,
            provider.clone(),
        );
        let call_builder = instance.getLatestStakeUpdate(operator_id, quorum_number);
        let response = call_builder
            .call()
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(response._0)
    }

    /// Get an Operator's ID as [`FixedBytes`] from its [`Address`].
    async fn get_operator_id(
        &self,
        operator_addr: Address,
    ) -> Result<alloy_primitives::FixedBytes<32>, std::io::Error> {
        self.avs_registry_reader()
            .await?
            .get_operator_id(operator_addr)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    /// Get the total stake at a given block number from a given index.
    async fn get_total_stake_at_block_number_from_index(
        &self,
        quorum_number: u8,
        block_number: u32,
        index: alloy_primitives::U256,
    ) -> Result<alloy_primitives::Uint<96, 2>, std::io::Error> {
        use alloy_provider::Provider as _;
        let contract_addresses = self
            .config
            .protocol_settings
            .eigenlayer()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let provider = self.get_provider_http()?;
        let registry_coordinator = eigensdk::utils::registrycoordinator::RegistryCoordinator::new(
            contract_addresses.registry_coordinator_address,
            provider.clone(),
        );
        let stake_registry_address = registry_coordinator
            .stakeRegistry()
            .call()
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
            ._0;
        let instance = eigensdk::utils::stakeregistry::StakeRegistry::StakeRegistryInstance::new(
            stake_registry_address,
            provider.clone(),
        );
        let call_builder =
            instance.getTotalStakeAtBlockNumberFromIndex(quorum_number, block_number, index);
        let response = call_builder
            .call()
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(response._0)
    }

    /// Get the total stake history length of a given quorum.
    async fn get_total_stake_history_length(
        &self,
        quorum_number: u8,
    ) -> Result<alloy_primitives::U256, std::io::Error> {
        let contract_addresses = self
            .config
            .protocol_settings
            .eigenlayer()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let provider = self.get_provider_http()?;
        let registry_coordinator = eigensdk::utils::registrycoordinator::RegistryCoordinator::new(
            contract_addresses.registry_coordinator_address,
            provider.clone(),
        );
        let stake_registry_address = registry_coordinator
            .stakeRegistry()
            .call()
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
            ._0;
        let instance = eigensdk::utils::stakeregistry::StakeRegistry::StakeRegistryInstance::new(
            stake_registry_address,
            provider.clone(),
        );
        let call_builder = instance.getTotalStakeHistoryLength(quorum_number);
        let response = call_builder
            .call()
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(response._0)
    }

    /// Provides the public keys of existing registered operators within the provided block range.
    async fn query_existing_registered_operator_pub_keys(
        &self,
        start_block: u64,
        to_block: u64,
    ) -> Result<
        (
            Vec<Address>,
            Vec<eigensdk::types::operator::OperatorPubKeys>,
        ),
        std::io::Error,
    > {
        let ws_rpc_endpoint = self.config.ws_rpc_endpoint.clone();
        self.avs_registry_reader()
            .await?
            .query_existing_registered_operator_pub_keys(start_block, to_block, ws_rpc_endpoint)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }
}
