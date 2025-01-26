use gadget_std::string::String;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum Sr25519Error {
    #[error("Invalid seed: {0}")]
    InvalidSeed(String),
    #[error("Invalid signature: {0}")]
    SignatureError(schnorrkel::SignatureError),
    #[error("Invalid hex string: {0}")]
    HexError(hex::FromHexError),
}

impl From<schnorrkel::SignatureError> for Sr25519Error {
    fn from(error: schnorrkel::SignatureError) -> Self {
        Sr25519Error::SignatureError(error)
    }
}

impl From<hex::FromHexError> for Sr25519Error {
    fn from(error: hex::FromHexError) -> Self {
        Sr25519Error::HexError(error)
    }
}

pub type Result<T> = gadget_std::result::Result<T, Sr25519Error>;
