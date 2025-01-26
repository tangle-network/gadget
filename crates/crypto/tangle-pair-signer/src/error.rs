use gadget_crypto_sp_core::error::SecretStringErrorWrapper;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TanglePairSignerError {
    #[error("Invalid secret string: {0}")]
    SecretStringError(SecretStringErrorWrapper),
    #[cfg(feature = "evm")]
    #[error("Ecdsa k256 error: {0}")]
    EcdsaError(#[from] k256::ecdsa::Error),
}

pub type Result<T> = gadget_std::result::Result<T, TanglePairSignerError>;
