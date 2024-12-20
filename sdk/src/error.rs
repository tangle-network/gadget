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

    #[error("Config error: {0}")]
    Config(#[from] crate::config::Error),

    #[error("Job runner error: {0}")]
    Runner(#[from] crate::runners::RunnerError),

    #[error("Executor error: {0}")]
    Executor(#[from] crate::executor::process::Error),

    #[error("Docker error: {0}")]
    Docker(#[from] crate::docker::Error),

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

    #[error("{0}")]
    Json(#[from] serde_json::Error),

    #[error("{0}")]
    BlueprintSerde(#[from] blueprint_serde::error::Error),

    #[cfg(feature = "std")]
    #[error("Prometheus error: {err}")]
    Prometheus { err: String },

    #[cfg(feature = "std")]
    #[error("Metrics error: {0}")]
    Metrics(#[from] crate::metrics::Error),

    #[error("Io error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("The type has been skipped in the preprocessor")]
    SkipPreProcessedType,

    #[error("Bad argument decoding for {0}")]
    BadArgumentDecoding(String),

    #[error("Color Eyre error: {0}")]
    Generic(#[from] color_eyre::Report),

    #[error("Other error: {0}")]
    Other(String),
}

impl From<bollard::errors::Error> for Error {
    fn from(error: bollard::errors::Error) -> Self {
        Error::Docker(crate::docker::Error::from(error))
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::Other(s)
    }
}
