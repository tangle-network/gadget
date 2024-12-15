pub mod runtime;
pub mod services;
pub mod tangle;

use std::io;

/// Error type for Tangle client operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An IO error occurred
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    /// A Subxt error occurred
    #[error("Subxt error: {0}")]
    Subxt(#[from] subxt::Error),

    /// A Tangle error occurred
    #[error("Tangle error: {0}")]
    Tangle(TangleDispatchError),
    /// A Other error occurred
    #[error("{0}")]
    Other(String),
}

#[derive(Debug)]
pub struct TangleDispatchError(
    pub tangle_subxt::tangle_testnet_runtime::api::runtime_types::sp_runtime::DispatchError,
);

impl From<tangle_subxt::tangle_testnet_runtime::api::runtime_types::sp_runtime::DispatchError>
    for TangleDispatchError
{
    fn from(
        error: tangle_subxt::tangle_testnet_runtime::api::runtime_types::sp_runtime::DispatchError,
    ) -> Self {
        TangleDispatchError(error)
    }
}

impl From<TangleDispatchError> for Error {
    fn from(error: TangleDispatchError) -> Self {
        Error::Tangle(error)
    }
}

impl gadget_std::fmt::Display for TangleDispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
