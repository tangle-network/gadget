use gadget_std::io;
use gadget_std::string::String;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TangleClientError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Subxt error: {0}")]
    Subxt(#[from] subxt::Error),
    #[error("Tangle error: {0}")]
    Tangle(TangleDispatchError),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = gadget_std::result::Result<T, TangleClientError>;

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

impl From<TangleDispatchError> for TangleClientError {
    fn from(error: TangleDispatchError) -> Self {
        TangleClientError::Tangle(error)
    }
}

impl gadget_std::fmt::Display for TangleDispatchError {
    fn fmt(&self, f: &mut gadget_std::fmt::Formatter<'_>) -> gadget_std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
