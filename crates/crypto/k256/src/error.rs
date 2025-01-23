use gadget_std::string::String;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum K256Error {
    #[error("Invalid seed: {0}")]
    InvalidSeed(String),
    #[error("Invalid verifying key: {0}")]
    InvalidVerifyingKey(String),
    #[error("Invalid signing key: {0}")]
    InvalidSigner(String),
    #[error("Invalid hex string: {0}")]
    HexError(hex::FromHexError),
    #[error("Invalid signature: {0}")]
    InvalidSignature(String),
    #[error("Signature failed: {0}")]
    SignatureFailed(String),
}

impl From<hex::FromHexError> for K256Error {
    fn from(error: hex::FromHexError) -> Self {
        K256Error::HexError(error)
    }
}

pub type Result<T> = gadget_std::result::Result<T, K256Error>;
