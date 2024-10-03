use alloc::string::String;
use sp_core::ecdsa;
use thiserror::Error;

/// Represents errors that can occur in the SDK.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Client error: {0}")]
    Client(String),

    #[error("Job error: {reason}")]
    Job { reason: String },

    #[error("Network error: {reason}")]
    Network { reason: String },

    #[error("Storage error: {reason}")]
    Store { reason: String },

    #[error("Keystore error: {0}")]
    Keystore(#[from] crate::keystore::error::Error),

    #[error("Invalid configuration: {0}")]
    ConfigError(#[from] crate::config::Error),

    #[error("Missing network ID")]
    MissingNetworkId,

    #[error("Peer not found: {id:?}")]
    PeerNotFound { id: ecdsa::Public },

    #[cfg(feature = "std")]
    #[error("Join error: {0}")]
    Join(#[from] tokio::task::JoinError),

    // TODO: Add feature flag for substrate/tangle
    #[error("Subxt error: {0}")]
    #[cfg(any(feature = "std", feature = "wasm"))]
    Subxt(#[from] subxt::Error),

    #[cfg(feature = "std")]
    #[error("Events watcher error: {0}")]
    EventsWatcher(#[from] crate::events_watcher::error::Error),

    #[cfg(feature = "std")]
    #[error("Prometheus error: {err}")]
    Prometheus { err: String },

    #[cfg(feature = "std")]
    #[error("Metrics error: {0}")]
    Metrics(#[from] crate::metrics::Error),

    #[error("Other error: {0}")]
    Other(String),
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::Other(s)
    }
}
