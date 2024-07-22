//! Keystore Errors

/// Different errors that can occur in the [`crate::keystore`] module
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// An I/O error occurred
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// An error occurred during sr25519 module operation
    #[cfg(feature = "keystore-sr25519")]
    #[error("sr25519: {0}")]
    Sr25519(schnorrkel::errors::SignatureError),
    /// An error occurred during ecdsa module operation
    #[cfg(feature = "keystore-ecdsa")]
    #[error("ecdsa: {0}")]
    Ecdsa(k256::elliptic_curve::Error),
    /// An error occurred during ed25519 module operation
    #[cfg(feature = "keystore-ed25519")]
    #[error("ed25519: {0}")]
    Ed25519(ed25519_zebra::Error),
}

impl From<ed25519_zebra::Error> for Error {
    fn from(v: ed25519_zebra::Error) -> Self {
        Self::Ed25519(v)
    }
}

impl From<schnorrkel::errors::SignatureError> for Error {
    fn from(v: schnorrkel::errors::SignatureError) -> Self {
        Self::Sr25519(v)
    }
}
