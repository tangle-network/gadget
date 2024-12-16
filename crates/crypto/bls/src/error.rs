use gadget_std::string::String;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum BlsError {
    #[error("Invalid seed: {0}")]
    InvalidSeed(String),
    #[error("Invalid hex string: {0}")]
    HexError(hex::FromHexError),
}

impl From<hex::FromHexError> for BlsError {
    fn from(error: hex::FromHexError) -> Self {
        BlsError::HexError(error)
    }
}

pub type Result<T> = gadget_std::result::Result<T, BlsError>;
