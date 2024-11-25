use alloy_primitives::{Address, Bytes, FixedBytes, U256};
use eigensdk::client_avsregistry::reader::AvsRegistryChainReader;
use eigensdk::client_avsregistry::writer::AvsRegistryChainWriter;
use eigensdk::client_elcontracts::writer::Operator;
use eigensdk::services_avsregistry::chaincaller::AvsRegistryServiceChainCaller;
use eigensdk::services_blsaggregation::bls_agg::BlsAggregatorService;
use eigensdk::services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceInMemory;
use eigensdk::types::operator::OperatorPubKeys;
use eigensdk::utils::binding::OperatorStateRetriever;
use eigensdk::utils::binding::StakeRegistry::StakeUpdate;
use num_bigint::BigInt;
use std::collections::HashMap;

/// `EigenlayerContext` trait provides access to Eigenlayer utilities
#[async_trait::async_trait]
pub trait EigenlayerContext {
    /// Provides a reader for the AVS registry.
    async fn avs_registry_reader(
        &self,
    ) -> color_eyre::Result<AvsRegistryChainReader, std::io::Error>;

    /// Provides a writer for the AVS registry.
    async fn avs_registry_writer(
        &self,
        private_key: String,
    ) -> color_eyre::Result<AvsRegistryChainWriter, std::io::Error>;

    /// Provides an operator info service.
    async fn operator_info_service_in_memory(
        &self,
    ) -> color_eyre::Result<OperatorInfoServiceInMemory, std::io::Error>;

    /// Provides an AVS registry service chain caller.
    async fn avs_registry_service_chain_caller_in_memory(
        &self,
    ) -> color_eyre::Result<
        AvsRegistryServiceChainCaller<AvsRegistryChainReader, OperatorInfoServiceInMemory>,
        std::io::Error,
    >;

    /// Provides a BLS aggregation service.
    async fn bls_aggregation_service_in_memory(
        &self,
    ) -> color_eyre::Result<
        BlsAggregatorService<
            AvsRegistryServiceChainCaller<AvsRegistryChainReader, OperatorInfoServiceInMemory>,
        >,
        std::io::Error,
    >;

    /// Get Operator stake in Quorums at a given block.
    async fn get_operator_stake_in_quorums_at_block(
        &self,
        block_number: u32,
        quorum_numbers: Bytes,
    ) -> color_eyre::Result<Vec<Vec<OperatorStateRetriever::Operator>>, std::io::Error>;

    /// Get an Operator's stake in Quorums at current block.
    async fn get_operator_stake_in_quorums_at_current_block(
        &self,
        operator_id: FixedBytes<32>,
    ) -> color_eyre::Result<HashMap<u8, BigInt>, std::io::Error>;

    /// Get an Operator by ID.
    async fn get_operator_by_id(
        &self,
        operator_id: [u8; 32],
    ) -> color_eyre::Result<Address, std::io::Error>;

    /// Get an Operator stake history.
    async fn get_operator_stake_history(
        &self,
        operator_id: FixedBytes<32>,
        quorum_number: u8,
    ) -> color_eyre::Result<Vec<StakeUpdate>, std::io::Error>;

    /// Get an Operator stake update at a given index.
    async fn get_operator_stake_update_at_index(
        &self,
        quorum_number: u8,
        operator_id: FixedBytes<32>,
        index: U256,
    ) -> color_eyre::Result<StakeUpdate, std::io::Error>;

    /// Get an Operator's stake at a given block number.
    async fn get_operator_stake_at_block_number(
        &self,
        operator_id: FixedBytes<32>,
        quorum_number: u8,
        block_number: u32,
    ) -> color_eyre::Result<u128, std::io::Error>;

    /// Get an Operator's [`details`](OperatorDetails).
    async fn get_operator_details(
        &self,
        operator_addr: Address,
    ) -> color_eyre::Result<Operator, std::io::Error>;

    /// Get an Operator's latest stake update.
    async fn get_latest_stake_update(
        &self,
        operator_id: FixedBytes<32>,
        quorum_number: u8,
    ) -> color_eyre::Result<StakeUpdate, std::io::Error>;

    /// Get an Operator's ID as [`FixedBytes`] from its [`Address`].
    async fn get_operator_id(
        &self,
        operator_addr: Address,
    ) -> color_eyre::Result<FixedBytes<32>, std::io::Error>;

    /// Get the total stake at a given block number from a given index.
    async fn get_total_stake_at_block_number_from_index(
        &self,
        quorum_number: u8,
        block_number: u32,
        index: U256,
    ) -> color_eyre::Result<u128, std::io::Error>;

    /// Get the total stake history length of a given quorum.
    async fn get_total_stake_history_length(
        &self,
        quorum_number: u8,
    ) -> color_eyre::Result<U256, std::io::Error>;

    /// Provides the public keys of existing registered operators within the provided block range.
    async fn query_existing_registered_operator_pub_keys(
        &self,
        start_block: u64,
        to_block: u64,
    ) -> color_eyre::Result<(Vec<Address>, Vec<OperatorPubKeys>), std::io::Error>;
}
