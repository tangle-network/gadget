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
    /// Invalid remote config
    #[error("Invalid remote config")]
    InvalidConfig,
    /// Keystore lock error
    #[error("Keystore lock error")]
    KeystoreLockError,
    /// Key type not supported
    #[error("Key type not supported")]
    KeyTypeNotSupported,
    /// Key not found
    #[error("Key not found")]
    KeyNotFound,
    /// Invalid message length
    #[error("Invalid message length")]
    InvalidMessageLength,
    /// Invalid hex decoding
    #[error("Invalid hex decoding")]
    InvalidHexDecoding,
    /// Invalid seed
    #[error("Invalid seed: {0}")]
    InvalidSeed(String),
    /// Signature failed
    #[error("Signature failed: {0}")]
    SignatureFailed(String),
    /// Remote key fetch failed
    #[error("Remote key fetch failed: {0}")]
    RemoteKeyFetchFailed(String),
    /// Serde json error
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    /// An error occurred during sr25519 module operation
    #[error("sr25519: {0}")]
    #[cfg(feature = "sr25519-schnorrkel")]
    SchnorrkelSr25519(String),
    /// An error occurred during ecdsa module operation
    #[error("ecdsa: {0}")]
    #[cfg(feature = "ecdsa")]
    Ecdsa(String),
    /// An error occurred during ed25519 module operation
    #[error("ed25519: {0}")]
    #[cfg(feature = "zebra")]
    Ed25519(#[from] ed25519_zebra::Error),
    /// An error occurred during bls381 module operation
    #[error("bls381: {0}")]
    #[cfg(feature = "bls381")]
    Bls381(String),
    /// An error occurred during bls_bn254 module operation
    #[error("bls_bn254: {0}")]
    #[cfg(feature = "bn254")]
    BlsBn254(String),
    /// An error occurred during alloy_ecdsa module operation
    #[error("alloy_ecdsa: {0}")]
    #[cfg(feature = "evm")]
    Alloy(String),
    /// Other error
    #[error("{0}")]
    Other(String),
}

#[cfg(feature = "sr25519-schnorrkel")]
impl From<schnorrkel::errors::SignatureError> for Error {
    fn from(v: schnorrkel::errors::SignatureError) -> Self {
        Self::SchnorrkelSr25519(v.to_string())
    }
}
