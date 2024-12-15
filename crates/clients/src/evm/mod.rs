pub mod evm_provider;

use std::io;

/// Error type for EVM client operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An IO error occurred
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// A provider error occurred
    #[error("Provider error: {0}")]
    Provider(String),

    /// Other error occurred
    #[error("{0}")]
    Other(String),
}
