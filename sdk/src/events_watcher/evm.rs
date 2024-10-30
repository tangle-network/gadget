//! EVM Event Watcher Module

use crate::events_watcher::error::Error;
use alloy_network::{Ethereum, EthereumWallet};
use alloy_primitives::FixedBytes;
use alloy_provider::{
    fillers::{ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller},
    Identity, Provider, RootProvider,
};
use alloy_sol_types::SolEvent;
use alloy_transport::Transport;
use alloy_transport_http::{Client, Http};
use std::ops::Deref;

pub trait Config: Send + Sync + Clone + 'static {
    type TH: Transport + Clone + Send + Sync;
    type PH: Provider<Self::TH, Ethereum> + Clone + Send + Sync;
}

#[derive(Debug, Copy, Clone)]
pub struct DefaultNodeConfig {}

impl Config for DefaultNodeConfig {
    type TH = Http<Client>;
    type PH = FillProvider<
        JoinFill<
            JoinFill<JoinFill<JoinFill<Identity, GasFiller>, NonceFiller>, ChainIdFiller>,
            WalletFiller<EthereumWallet>,
        >,
        RootProvider<Http<Client>>,
        Http<Client>,
        Ethereum,
    >;
}

pub trait EvmContract<T: Config>:
    Deref<Target = alloy_contract::ContractInstance<T::TH, T::PH, Ethereum>>
    + Send
    + Clone
    + Sync
    + 'static
{
}
impl<
        T: Config,
        X: Deref<Target = alloy_contract::ContractInstance<T::TH, T::PH, Ethereum>>
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
pub trait EvmEventHandler<T: Config>: Send + Sync + 'static {
    /// The contract that this event watcher is watching.
    type Contract: EvmContract<T>;
    /// The type of event this handler is for.
    type Event: EvmEvent;
    /// The genesis transaction hash for the contract.
    const GENESIS_TX_HASH: FixedBytes<32>;
    /// Handle a log event.
    async fn handle(&self, log: &alloy_rpc_types::Log, event: &Self::Event) -> Result<(), Error>;
}
