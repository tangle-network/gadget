use alloc::string::String;
use thiserror::Error;

pub type Result<T> = core::result::Result<T, GadgetIoError>;

#[derive(Debug, Error)]
pub enum GadgetIoError {
    // External
    // TODO: Use this once `sp_core::crypto::SecretStringError` uses Error in core
    // #[error("Failed to parse secret string: {0}")]
    // SecretString(#[from] sp_core::crypto::SecretStringError),
    // TODO: Use this once `sc_keystore::Error` uses Error in core
    // #[error("Keystore error: {0}")]
    // KeyStore(#[from] sc_keystore::Error),
    #[error("{0}")]
    Other(String),
}

impl From<String> for GadgetIoError {
    fn from(s: String) -> Self {
        GadgetIoError::Other(s)
    }
}
