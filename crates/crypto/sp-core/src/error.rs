use gadget_std::string::String;
use sp_core::crypto::SecretStringError;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum SpCoreError {
    #[error("Invalid seed: {0}")]
    InvalidSeed(String),
    #[error("Invalid secret string: {0}")]
    SecretStringError(SecretStringErrorWrapper),
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Invalid public key")]
    InvalidPublicKey,
}

#[derive(Debug, Clone)]
pub struct SecretStringErrorWrapper(pub SecretStringError);

impl gadget_std::fmt::Display for SecretStringErrorWrapper {
    fn fmt(&self, f: &mut gadget_std::fmt::Formatter<'_>) -> gadget_std::fmt::Result {
        match &self.0 {
            SecretStringError::InvalidFormat(err) => write!(f, "Invalid format: {err}"),
            SecretStringError::InvalidPhrase => write!(f, "Invalid phrase"),
            SecretStringError::InvalidPassword => write!(f, "Invalid password"),
            SecretStringError::InvalidSeed => write!(f, "Invalid seed"),
            SecretStringError::InvalidSeedLength => write!(f, "Invalid seed length"),
            SecretStringError::InvalidPath => write!(f, "Invalid path"),
        }
    }
}

impl From<SecretStringError> for SecretStringErrorWrapper {
    fn from(e: SecretStringError) -> Self {
        SecretStringErrorWrapper(e)
    }
}

impl From<SecretStringErrorWrapper> for SpCoreError {
    fn from(e: SecretStringErrorWrapper) -> Self {
        SpCoreError::SecretStringError(e)
    }
}

pub type Result<T> = gadget_std::result::Result<T, SpCoreError>;
