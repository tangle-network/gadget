use alloc::boxed::Box;

/// An error type for the event watcher.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// An error occurred in Subxt.
    // TODO: Add feature flag for substrate/tangle
    #[error(transparent)]
    Subxt(#[from] subxt::Error),
    /// An error occurred in Alloy transport.
    // TODO: Add feature flag for EVM/eigenlayer/etc.
    #[error(transparent)]
    AlloyTransport(#[from] alloy_transport::TransportError),
    /// An error occurred in Alloy Contract.
    // TODO: Add feature flag for EVM/eigenlayer/etc.
    #[error(transparent)]
    AlloyContract(#[from] alloy_contract::Error),
    /// An error for sol types.
    /// TODO: Add feature flag for EVM/eigenlayer/etc.
    #[error(transparent)]
    SolTypes(#[from] alloy_sol_types::Error),
    /// An error occurred in the event watcher, and we need to restart it.
    #[error("An error occurred in the event watcher and we need to restart it.")]
    ForceRestart,
    /// An error occurred in the event handler.
    #[error(transparent)]
    Handler(#[from] Box<dyn core::error::Error + Send + Sync>),
}
