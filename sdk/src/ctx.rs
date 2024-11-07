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

use core::future::Future;

use crate::keystore::backend::GenericKeyStore;
use eigensdk::{
    client_avsregistry::{reader::AvsRegistryChainReader, writer::AvsRegistryChainWriter},
    services_avsregistry::chaincaller::AvsRegistryServiceChainCaller,
    services_blsaggregation::bls_agg::BlsAggregatorService,
    services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceInMemory,
};
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
}
