//! EVM Event Watcher Module

use crate::events_watcher::error::Error;
use alloy_network::{Ethereum, EthereumWallet};
use alloy_primitives::FixedBytes;
use alloy_provider::{Provider, ProviderBuilder, RootProvider, WsConnect};
use alloy_sol_types::SolEvent;
use alloy_transport::BoxTransport;
use std::ops::Deref;

pub trait EvmContract:
    Deref<
        Target = alloy_contract::ContractInstance<
            BoxTransport,
            RootProvider<BoxTransport>,
            Ethereum,
        >,
    > + Send
    + Clone
    + Sync
    + 'static
{
}
impl<
        X: Deref<
                Target = alloy_contract::ContractInstance<
                    BoxTransport,
                    RootProvider<BoxTransport>,
                    Ethereum,
                >,
            > + Send
            + Clone
            + Sync
            + 'static,
    > EvmContract for X
{
}

pub trait EvmEvent: SolEvent + Clone + Send + Sync + 'static {}
impl<X: SolEvent + Clone + Send + Sync + 'static> EvmEvent for X {}

/// A trait for watching events from a contract.
/// EventWatcher trait exists for deployments that are smart-contract / EVM based
#[async_trait::async_trait]
pub trait EvmEventHandler: Send + Sync + 'static {
    /// The contract that this event watcher is watching.
    type Contract: EvmContract;
    /// The type of event this handler is for.
    type Event: EvmEvent;
    /// The genesis transaction hash for the contract.
    const GENESIS_TX_HASH: FixedBytes<32>;
    /// Handle a log event.
    async fn handle(&self, log: &alloy_rpc_types::Log, event: &Self::Event) -> Result<(), Error>;
}

pub fn get_provider_http(http_endpoint: &str) -> RootProvider<BoxTransport> {
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .on_http(http_endpoint.parse().unwrap())
        .root()
        .clone()
        .boxed();

    provider
}

pub fn get_wallet_provider_http(
    http_endpoint: &str,
    wallet: EthereumWallet,
) -> RootProvider<BoxTransport> {
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_http(http_endpoint.parse().unwrap())
        .root()
        .clone()
        .boxed();

    provider
}

pub async fn get_provider_ws(ws_endpoint: &str) -> RootProvider<BoxTransport> {
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .on_ws(WsConnect::new(ws_endpoint))
        .await
        .unwrap()
        .root()
        .clone()
        .boxed();

    provider
}
