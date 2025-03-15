use gadget_std::boxed::Box;
/// Keystore Errors
use gadget_std::string::String;

pub type Result<T> = core::result::Result<T, Error>;

/// Different errors that can occur in the keystore
#[derive(Debug, thiserror::Error)]
#[rustfmt::skip]
#[non_exhaustive]
pub enum Error {
    /* Core errors, always available */

    /// Storage unsupported
    #[error("Storage not supported")]
    StorageNotSupported,
    /// An I/O error occurred
    #[error(transparent)]
    Io(#[from] gadget_std::io::Error),
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
    /// Failed to deserialize key
    #[error("Failed to deserialize key: {0}")]
    KeyDeserialization(#[from] serde::de::value::Error),
    /// Invalid seed
    #[error("Invalid seed: {0}")]
    InvalidSeed(String),
    /// Signature failed
    #[error("Signature failed: {0}")]
    SignatureFailed(String),
    /// Remote key fetch failed
    #[error("Remote key fetch failed: {0}")]
    RemoteKeyFetchFailed(String),

    /* Crypto errors */
    #[error(transparent)]
    Crypto(#[from] gadget_crypto::CryptoCoreError),

    /* Feature-specific errors */

    #[cfg(feature = "aws-signer")]
    #[error(transparent)]
    AwsSigner(#[from] alloy_signer_aws::AwsSignerError),
    #[cfg(feature = "gcp-signer")]
    #[error(transparent)]
    GCloud(#[from] gcloud_sdk::error::Error),
    #[cfg(feature = "gcp-signer")]
    #[error(transparent)]
    GcpSigner(#[from] alloy_signer_gcp::GcpSignerError),
    #[cfg(any(feature = "ledger-node", feature = "ledger-browser"))]
    #[error(transparent)]
    Ledger(#[from] alloy_signer_ledger::LedgerError),
    /// Secret string error
    #[cfg(feature = "tangle")]
    #[error(transparent)]
    SecretStringError(#[from] sp_core::crypto::SecretStringError),
    /// Serde json error
    #[error(transparent)]
    #[cfg(feature = "std")]
    SerdeJsonError(#[from] serde_json::Error),
    /// An error occurred during sr25519 module operation
    #[error("sr25519: {0}")]
    #[cfg(feature = "sr25519-schnorrkel")]
    SchnorrkelSr25519(String),
    /// An error occurred during ecdsa module operation
    #[error("ecdsa: {0}")]
    #[cfg(feature = "ecdsa")]
    Ecdsa(#[from] k256::ecdsa::Error),
    /// An error occurred during ed25519 module operation
    #[error("ed25519: {0}")]
    #[cfg(feature = "zebra")]
    Ed25519(#[from] ed25519_zebra::Error),
    /// An error occurred during bls381 module operation
    #[error("bls: {0}")]
    #[cfg(feature = "bls")]
    Bls(String),
    /// An error occurred during bn254 module operation
    #[error("bls_bn254: {0}")]
    #[cfg(feature = "bn254")]
    BlsBn254(String),
    /// Other error
    #[error("{0}")]
    Other(String),

    /// Alloy signer error
    #[error(transparent)]
    #[cfg(feature = "evm")]
    AlloySigner(#[from] alloy_signer::Error),
    /// Alloy local signer error
    #[error(transparent)]
    #[cfg(feature = "evm")]
    LocalSignerError(#[from] alloy_signer_local::LocalSignerError),
}

#[macro_export]
macro_rules! impl_from_for_boxed_error {
    ($error_type:ty, $variant:ident) => {
        impl From<$error_type> for Box<Error> {
            fn from(e: $error_type) -> Self {
                Box::new(Error::$variant(e))
            }
        }
    };
}

impl_from_for_boxed_error!(gadget_std::io::Error, Io);
impl_from_for_boxed_error!(serde::de::value::Error, KeyDeserialization);
impl_from_for_boxed_error!(gadget_crypto::CryptoCoreError, Crypto);
#[cfg(feature = "aws-signer")]
impl_from_for_boxed_error!(alloy_signer_aws::AwsSignerError, AwsSigner);
#[cfg(feature = "gcp-signer")]
impl_from_for_boxed_error!(gcloud_sdk::error::Error, GCloud);
#[cfg(feature = "gcp-signer")]
impl_from_for_boxed_error!(alloy_signer_gcp::GcpSignerError, GcpSigner);
#[cfg(any(feature = "ledger-node", feature = "ledger-browser"))]
impl_from_for_boxed_error!(alloy_signer_ledger::LedgerError, Ledger);
#[cfg(feature = "tangle")]
impl_from_for_boxed_error!(sp_core::crypto::SecretStringError, SecretStringError);
#[cfg(feature = "std")]
impl_from_for_boxed_error!(serde_json::Error, SerdeJsonError);
#[cfg(feature = "ecdsa")]
impl_from_for_boxed_error!(k256::ecdsa::Error, Ecdsa);
#[cfg(feature = "zebra")]
impl_from_for_boxed_error!(ed25519_zebra::Error, Ed25519);
#[cfg(feature = "evm")]
impl_from_for_boxed_error!(alloy_signer::Error, AlloySigner);
#[cfg(feature = "evm")]
impl_from_for_boxed_error!(alloy_signer_local::LocalSignerError, LocalSignerError);

#[cfg(feature = "sr25519-schnorrkel")]
impl From<schnorrkel::errors::SignatureError> for Error {
    fn from(v: schnorrkel::errors::SignatureError) -> Self {
        Self::SchnorrkelSr25519(v.to_string())
    }
}
