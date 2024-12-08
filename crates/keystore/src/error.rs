//! Keystore Errors
use gadget_std::string::String;

/// Different errors that can occur in the [`crate::keystore`] module
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// An I/O error occurred
    #[error(transparent)]
    #[cfg(feature = "std")]
    Io(#[from] std::io::Error),
    /// An error occurred during sr25519 module operation
    #[error("sr25519: {0}")]
    #[cfg(feature = "sr25519")]
    Sr25519(String),
    /// An error occurred during ecdsa module operation
    #[error("ecdsa: {0}")]
    #[cfg(feature = "ecdsa")]
    Ecdsa(String),
    /// An error occurred during ed25519 module operation
    #[error("ed25519: {0}")]
    #[cfg(feature = "ed25519")]
    Ed25519(String),
    /// An error occurred during bls381 module operation
    #[error("bls381: {0}")]
    #[cfg(feature = "bls381")]
    Bls381(String),
    /// An error occurred during bls_bn254 module operation
    #[error("bls_bn254: {0}")]
    #[cfg(feature = "bn254")]
    BlsBn254(String),
    /// An error occurred during bls_bn254 module operation
    #[error("alloy_ecdsa: {0}")]
    #[cfg(feature = "evm")]
    Alloy(String),
}

#[cfg(feature = "ed25519")]
impl From<ed25519_zebra::Error> for Error {
    fn from(v: ed25519_zebra::Error) -> Self {
        Self::Ed25519(v.to_string())
    }
}

#[cfg(feature = "sr25519")]
impl From<schnorrkel::errors::SignatureError> for Error {
    fn from(v: schnorrkel::errors::SignatureError) -> Self {
        Self::Sr25519(v.to_string())
    }
}
