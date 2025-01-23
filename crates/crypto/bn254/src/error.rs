use gadget_std::string::String;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum Bn254Error {
    #[error("Invalid seed: {0}")]
    InvalidSeed(String),
    #[error("Invalid signature: {0}")]
    SignatureFailed(String),
    #[error("Signature not in subgroup")]
    SignatureNotInSubgroup,
}

pub type Result<T> = gadget_std::result::Result<T, Bn254Error>;
