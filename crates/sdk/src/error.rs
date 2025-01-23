use super::{config, keystore};

#[cfg(any(feature = "evm", feature = "eigenlayer", feature = "tangle"))]
use super::event_listeners;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // General Errors
    #[error("Config error: {0}")]
    Config(#[from] config::Error),
    #[error("Keystore error: {0}")]
    Keystore(#[from] keystore::Error),

    // Specific to Tangle
    #[cfg(feature = "tangle")]
    #[error("Event listener error: {0}")]
    TangleEvent(
        #[from]
        event_listeners::core::Error<event_listeners::tangle::error::TangleEventListenerError>,
    ),
    #[cfg(feature = "tangle")]
    #[error("Tangle Subxt error: {0}")]
    TangleSubxt(#[from] tangle_subxt::subxt::Error),

    // EVM and EigenLayer
    #[cfg(any(feature = "evm", feature = "eigenlayer"))]
    #[error("Event listener error: {0}")]
    EvmEvent(#[from] event_listeners::core::Error<event_listeners::evm::error::Error>),
    #[cfg(any(feature = "evm", feature = "eigenlayer"))]
    #[error("EVM error: {0}")]
    Alloy(#[from] AlloyError),
    #[cfg(feature = "eigenlayer")]
    #[error("Eigenlayer error: {0}")]
    Eigenlayer(#[from] eigensdk::types::avs::SignatureVerificationError),

    // Specific to Networking
    #[cfg(feature = "networking")]
    #[error("Networking error: {0}")]
    Networking(#[from] gadget_networking::Error),
}

#[cfg(any(feature = "evm", feature = "eigenlayer"))]
#[derive(thiserror::Error, Debug)]
pub enum AlloyError {
    #[error("Alloy signer error: {0}")]
    Signer(#[from] alloy::signers::Error),
    #[error("Alloy contract error: {0}")]
    Contract(#[from] alloy::contract::Error),
    #[error("Alloy transaction error: {0}")]
    Conversion(#[from] alloy::rpc::types::transaction::ConversionError),
    #[error("Alloy local signer error: {0}")]
    LocalSigner(#[from] alloy::signers::local::LocalSignerError),
}

#[cfg(any(feature = "evm", feature = "eigenlayer"))]
macro_rules! implement_from_alloy_error {
    ($($path:ident)::+, $variant:ident) => {
        impl From<alloy::$($path)::+> for Error {
            fn from(value: alloy::$($path)::+) -> Self {
                Error::Alloy(AlloyError::$variant(value))
            }
        }
    };
}

#[cfg(any(feature = "evm", feature = "eigenlayer"))]
implement_from_alloy_error!(signers::Error, Signer);
#[cfg(any(feature = "evm", feature = "eigenlayer"))]
implement_from_alloy_error!(contract::Error, Contract);
#[cfg(any(feature = "evm", feature = "eigenlayer"))]
implement_from_alloy_error!(rpc::types::transaction::ConversionError, Conversion);
#[cfg(any(feature = "evm", feature = "eigenlayer"))]
implement_from_alloy_error!(signers::local::LocalSignerError, LocalSigner);

