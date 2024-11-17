//! A set of traits and utilities that provide a common interface for interacting with the Gadget SDK.
//!
//! Usually, when you need access to the SDK, you will need to pass the Context to your jobs/functions. In your code, you will create a struct that encapsulates all the things that you would need from outside world from your job.
//! for example, if you need to interact with the network, you will need to have a network client in your struct. If you need to interact with the database storage, you will need to have a db client in your struct. And so on.
//!
//! This module provides a set of traits that you can implement for your struct to make it a context-aware struct by adding new functionalities to it.
//!
//! # Example
//!
//! ```rust,no_run
//! use gadget_sdk::config::StdGadgetConfiguration;
//! use gadget_sdk::ctx::KeystoreContext;
//! use gadget_sdk::event_listener::tangle::jobs::{
//!     services_post_processor, services_pre_processor,
//! };
//! use gadget_sdk::event_listener::tangle::TangleEventListener;
//! use gadget_sdk::job;
//! use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::services::events::JobCalled;
//!
//! // This your struct that encapsulates all the things you need from outside world.
//! // By deriving KeystoreContext, you can now access the keystore client from your struct.
//! #[derive(Clone, Debug, KeystoreContext)]
//! struct MyContext {
//!     foo: String,
//!     bar: u64,
//!     #[config]
//!     sdk_config: StdGadgetConfiguration,
//! }
//!
//! #[job(
//!     id = 0,
//!     params(who),
//!     result(_),
//!     event_listener(
//!         listener = TangleEventListener<MyContext, JobCalled>,
//!         pre_processor = services_pre_processor,
//!         post_processor = services_post_processor,
//!     )
//! )]
//! async fn my_job(who: String, ctx: MyContext) -> Result<String, std::convert::Infallible> {
//!     // Access the keystore client from the context.
//!     let keystore = ctx.keystore();
//!     // Do something with the keystore client.
//!     // ...
//!     Ok(format!("Hello, {}!", who))
//! }
//! ```

use crate::keystore::backend::GenericKeyStore;
use alloy_primitives::{Address, Bytes, FixedBytes, U256};
use tangle_subxt::tangle_testnet_runtime::api::assets::events::accounts_destroyed::AssetId;
use tangle_subxt::tangle_testnet_runtime::api::assets::events::burned::Balance;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::pallet_multi_asset_delegation::types::delegator::DelegatorMetadata;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::pallet_multi_asset_delegation::types::operator::OperatorMetadata;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::Service;
use tangle_subxt::tangle_testnet_runtime::api::system::storage::types::number::Number;
use core::future::Future;
use eigensdk::types::operator::{Operator, OperatorPubKeys};
use eigensdk::utils::binding::OperatorStateRetriever;
use eigensdk::utils::binding::StakeRegistry::StakeUpdate;
use eigensdk::{
    client_avsregistry::{reader::AvsRegistryChainReader, writer::AvsRegistryChainWriter},
    services_avsregistry::chaincaller::AvsRegistryServiceChainCaller,
    services_blsaggregation::bls_agg::BlsAggregatorService,
    services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceInMemory,
};
pub use num_bigint::BigInt;
use std::collections::HashMap;
// derives
pub use gadget_context_derive::*;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::{
    sp_arithmetic::per_things::Percent, tangle_primitives::services::ServiceBlueprint,
};

/// `KeystoreContext` trait provides access to the generic keystore from the context.
pub trait KeystoreContext<RwLock: lock_api::RawRwLock> {
    /// Get the keystore client from the context.
    fn keystore(&self) -> Result<GenericKeyStore<RwLock>, crate::config::Error>;
}

/// `GossipNetworkContext` trait provides access to the network client from the context.
pub trait GossipNetworkContext {
    /// Get the Goossip client from the context.
    fn gossip_network(&self) -> &crate::network::gossip::GossipHandle;
}

/// `EVMProviderContext` trait provides access to the EVM provider from the context.
pub trait EVMProviderContext {
    type Network: alloy_network::Network;
    type Transport: alloy_transport::Transport + Clone;
    type Provider: alloy_provider::Provider<Self::Transport, Self::Network>;
    /// Get the EVM provider from the context.
    fn evm_provider(
        &self,
    ) -> impl Future<Output = Result<Self::Provider, alloy_transport::TransportError>>;
}

/// `TangleClientContext` trait provides access to the Tangle client from the context.
pub trait TangleClientContext {
    type Config: subxt::Config;
    /// Get the Tangle client from the context.
    fn tangle_client(
        &self,
    ) -> impl Future<Output = Result<subxt::OnlineClient<Self::Config>, subxt::Error>>;
}

/// `ServicesContext` trait provides access to the current service and current blueprint from the context.
pub trait ServicesContext {
    type Config: subxt::Config;
    /// Get the current blueprint information from the context.
    fn current_blueprint(
        &self,
        client: &subxt::OnlineClient<Self::Config>,
    ) -> impl Future<Output = Result<ServiceBlueprint, subxt::Error>>;

    /// Query the current blueprint owner from the context.
    fn current_blueprint_owner(
        &self,
        client: &subxt::OnlineClient<Self::Config>,
    ) -> impl Future<Output = Result<subxt::utils::AccountId32, subxt::Error>>;

    /// Get the current service operators with their restake exposure  from the context.
    /// This function will return a list of service operators that are selected to run this service
    /// instance.
    fn current_service_operators(
        &self,
        client: &subxt::OnlineClient<Self::Config>,
    ) -> impl Future<Output = Result<Vec<(subxt::utils::AccountId32, Percent)>, subxt::Error>>;

    #[allow(clippy::type_complexity)]
    /// Get metadata for a list of operators from the context.
    fn get_operators_metadata(
        &self,
        client: &subxt::OnlineClient<Self::Config>,
        operators: Vec<subxt::utils::AccountId32>,
    ) -> impl Future<
        Output = Result<
            Vec<(
                subxt::utils::AccountId32,
                OperatorMetadata<subxt::utils::AccountId32, Balance, AssetId>,
            )>,
            subxt::Error,
        >,
    >;

    /// Get metadata for a single operator from the context.
    /// This function will return the metadata for a single operator.
    fn get_operator_metadata(
        &self,
        client: &subxt::OnlineClient<Self::Config>,
        operator: subxt::utils::AccountId32,
    ) -> impl Future<
        Output = Result<
            Option<OperatorMetadata<subxt::utils::AccountId32, Balance, AssetId>>,
            subxt::Error,
        >,
    >;

    /// Get the current service instance from the context.
    fn get_service_instance(
        &self,
        client: &subxt::OnlineClient<Self::Config>,
    ) -> impl Future<Output = Result<Service<subxt::utils::AccountId32, Number, AssetId>, subxt::Error>>;

    #[allow(clippy::type_complexity)]
    /// Get delegations for a list of operators from the context.
    fn get_operator_delegations(
        &self,
        client: &subxt::OnlineClient<Self::Config>,
        operators: Vec<subxt::utils::AccountId32>,
    ) -> impl Future<
        Output = Result<
            Vec<(
                subxt::utils::AccountId32, // operator
                Option<DelegatorMetadata<subxt::utils::AccountId32, AssetId, Balance>>,
            )>,
            subxt::Error,
        >,
    >;

    /// Get delegations for a single operator from the context.
    fn get_operator_delegation(
        &self,
        client: &subxt::OnlineClient<Self::Config>,
        operator: subxt::utils::AccountId32,
    ) -> impl Future<
        Output = Result<
            Option<DelegatorMetadata<subxt::utils::AccountId32, AssetId, Balance>>,
            subxt::Error,
        >,
    >;
}

/// `EigenlayerContext` trait provides access to Eigenlayer utilities
#[async_trait::async_trait]
pub trait EigenlayerContext {
    /// Provides a reader for the AVS registry.
    async fn avs_registry_reader(&self) -> Result<AvsRegistryChainReader, std::io::Error>;

    /// Provides a writer for the AVS registry.
    async fn avs_registry_writer(
        &self,
        private_key: String,
    ) -> Result<AvsRegistryChainWriter, std::io::Error>;

    /// Provides an operator info service.
    async fn operator_info_service_in_memory(
        &self,
    ) -> Result<OperatorInfoServiceInMemory, std::io::Error>;

    /// Provides an AVS registry service chain caller.
    async fn avs_registry_service_chain_caller_in_memory(
        &self,
    ) -> Result<
        AvsRegistryServiceChainCaller<AvsRegistryChainReader, OperatorInfoServiceInMemory>,
        std::io::Error,
    >;

    /// Provides a BLS aggregation service.
    async fn bls_aggregation_service_in_memory(
        &self,
    ) -> Result<
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
    ) -> Result<Vec<Vec<OperatorStateRetriever::Operator>>, std::io::Error>;

    /// Get an Operator's stake in Quorums at current block.
    async fn get_operator_stake_in_quorums_at_current_block(
        &self,
        operator_id: FixedBytes<32>,
    ) -> Result<HashMap<u8, BigInt>, std::io::Error>;

    /// Get an Operator by ID.
    async fn get_operator_by_id(&self, operator_id: [u8; 32]) -> Result<Address, std::io::Error>;

    /// Get an Operator stake history.
    async fn get_operator_stake_history(
        &self,
        operator_id: FixedBytes<32>,
        quorum_number: u8,
    ) -> Result<Vec<StakeUpdate>, std::io::Error>;

    /// Get an Operator stake update at a given index.
    async fn get_operator_stake_update_at_index(
        &self,
        quorum_number: u8,
        operator_id: FixedBytes<32>,
        index: U256,
    ) -> Result<StakeUpdate, std::io::Error>;

    /// Get an Operator's stake at a given block number.
    async fn get_operator_stake_at_block_number(
        &self,
        operator_id: FixedBytes<32>,
        quorum_number: u8,
        block_number: u32,
    ) -> Result<u128, std::io::Error>;

    /// Get an Operator's [`details`](OperatorDetails).
    async fn get_operator_details(
        &self,
        operator_addr: Address,
    ) -> Result<Operator, std::io::Error>;

    /// Get an Operator's latest stake update.
    async fn get_latest_stake_update(
        &self,
        operator_id: FixedBytes<32>,
        quorum_number: u8,
    ) -> Result<StakeUpdate, std::io::Error>;

    /// Get an Operator's ID as [`FixedBytes`] from its [`Address`].
    async fn get_operator_id(
        &self,
        operator_addr: Address,
    ) -> Result<FixedBytes<32>, std::io::Error>;

    /// Get the total stake at a given block number from a given index.
    async fn get_total_stake_at_block_number_from_index(
        &self,
        quorum_number: u8,
        block_number: u32,
        index: U256,
    ) -> Result<u128, std::io::Error>;

    /// Get the total stake history length of a given quorum.
    async fn get_total_stake_history_length(
        &self,
        quorum_number: u8,
    ) -> Result<U256, std::io::Error>;

    /// Provides the public keys of existing registered operators within the provided block range.
    async fn query_existing_registered_operator_pub_keys(
        &self,
        start_block: u64,
        to_block: u64,
    ) -> Result<(Vec<Address>, Vec<OperatorPubKeys>), std::io::Error>;
}
