use gadget_std::string::String;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum Ed25519Error {
    #[error("Invalid seed: {0}")]
    InvalidSeed(String),
    #[error("Invalid public key: {0}")]
    InvalidVerifyingKey(String),
    #[error("Invalid signing key: {0}")]
    InvalidSigner(String),
    #[error("Invalid signature: {0}")]
    InvalidSignature(String),
    #[error("Zebra error: {0}")]
    ZebraError(ed25519_zebra::Error),
    #[error("Invalid hex string: {0}")]
    HexError(hex::FromHexError),
}

impl From<ed25519_zebra::Error> for Ed25519Error {
    fn from(error: ed25519_zebra::Error) -> Self {
        Ed25519Error::ZebraError(error)
    }
}

impl From<hex::FromHexError> for Ed25519Error {
    fn from(error: hex::FromHexError) -> Self {
        Ed25519Error::HexError(error)
    }
}

pub type Result<T> = gadget_std::result::Result<T, Ed25519Error>;
