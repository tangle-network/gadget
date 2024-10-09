//! EVM Event Watcher Module

use crate::events_watcher::error::Error;
use crate::events_watcher::InitializableEventHandler;
use alloy_network::{Ethereum, Network};
use alloy_primitives::FixedBytes;
use alloy_provider::Provider;
use alloy_sol_types::SolEvent;
use alloy_transport::Transport;
use async_trait::async_trait;
use std::ops::Deref;

pub trait Config: Send + Sync + Clone + 'static {
    type TH: Transport + Clone + Send + Sync;
    type PH: Provider<Self::TH, Ethereum> + Clone + Send + Sync;
}

pub trait EvmContract<T: Config<N = Ethereum>>:
    Deref<Target = alloy_contract::ContractInstance<T::T, T::P, Ethereum>>
    + Send
    + Clone
    + Sync
    + 'static
{
}
impl<
        T: Config<N = Ethereum>,
        X: Deref<Target = alloy_contract::ContractInstance<T::T, T::P, Ethereum>>
            + Send
            + Clone
            + Sync
            + 'static,
    > EvmContract<T> for X
{
}

pub trait EvmEvent: SolEvent + Clone + Send + Sync + 'static {}
impl<X: SolEvent + Clone + Send + Sync + 'static> EvmEvent for X {}

/// A trait for watching events from a contract.
/// EventWatcher trait exists for deployments that are smart-contract / EVM based
#[async_trait::async_trait]
pub trait EvmEventHandler<T: Config<N = Ethereum>>: Send + Sync + 'static {
    /// A Helper tag used to identify the event watcher during the logs.
    const TAG: &'static str;
    /// The contract that this event watcher is watching.
    type Contract: EvmContract<T>;
    /// The type of event this handler is for.
    type Event: EvmEvent;
    /// The genesis transaction hash for the contract.
    const GENESIS_TX_HASH: FixedBytes<32>;
    async fn init(&self) -> Option<tokio::sync::oneshot::Receiver<()>>;
    // (Self::Event, alloy_rpc_types::Log)
    async fn handle(&self, log: &alloy_rpc_types::Log, event: &Self::Event) -> Result<(), Error>;
}

#[async_trait]
impl<T: Config<N = Ethereum>, Handler: EvmEventHandler<T>> InitializableEventHandler<T>
    for Handler
{
    async fn init_event_handler(&self) -> Option<tokio::sync::oneshot::Receiver<()>> {
        self.init().await
    }
}
