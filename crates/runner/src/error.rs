use blueprint_core::error::BoxError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RunnerError {
    #[error("Protocol error: {0}")]
    InvalidProtocol(String),

    #[error("Keystore error: {0}")]
    Keystore(String),

    #[error("Signature error: {0}")]
    SignatureError(String),

    #[error("Transaction error: {0}")]
    TransactionError(String),

    #[error("Not an active operator")]
    NotActiveOperator,

    #[error("Receive error: {0}")]
    Recv(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Tangle error: {0}")]
    Tangle(String),

    #[error("Blueprint runner configured without a router")]
    NoRouter,

    #[error("A background service failed: {0}")]
    BackgroundService(String),

    #[error("A job call failed: {0}")]
    JobCall(String),

    #[error("A consumer failed: {0}")]
    Consumer(BoxError),

    #[error("Generic error: {0}")]
    Other(String),

    #[error("EigenLayer error: {0}")]
    Eigenlayer(String),

    #[error("Contract error: {0}")]
    Contract(String),

    #[error("AVS Registry error: {0}")]
    AvsRegistry(String),
}

// Convenience Result type
pub type Result<T> = std::result::Result<T, RunnerError>;

/// Errors that can occur while loading and using the blueprint configuration.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ConfigError {
    /// Missing `RPC_URL` environment variable.
    #[error("Missing Tangle RPC endpoint")]
    MissingTangleRpcEndpoint,
    /// Missing `KEYSTORE_URI` environment
    #[error("Missing keystore URI")]
    MissingKeystoreUri,
    /// Missing `BLUEPRINT_ID` environment variable
    #[error("Missing blueprint ID")]
    MissingBlueprintId,
    /// Missing `SERVICE_ID` environment variable
    #[error("Missing service ID")]
    MissingServiceId,
    /// Error parsing the blueprint ID.
    #[error(transparent)]
    MalformedBlueprintId(core::num::ParseIntError),
    /// Error parsing the service ID.
    #[error(transparent)]
    MalformedServiceId(core::num::ParseIntError),
    /// Unsupported keystore URI.
    #[error("Unsupported keystore URI: {0}")]
    UnsupportedKeystoreUri(String),
    /// Error parsing the protocol, from the `PROTOCOL` environment variable.
    #[error("Unsupported protocol: {0}")]
    UnsupportedProtocol(String),
    /// Attempting to load the [`ProtocolSettings`] of a protocol differing from the target
    ///
    /// [`ProtocolSettings`]: crate::ProtocolSettings
    #[error("Unexpect protocol, expected {0}")]
    UnexpectedProtocol(&'static str),
    /// No Sr25519 keypair found in the keystore.
    #[error("No Sr25519 keypair found in the keystore")]
    NoSr25519Keypair,
    /// Invalid Sr25519 keypair found in the keystore.
    #[error("Invalid Sr25519 keypair found in the keystore")]
    InvalidSr25519Keypair,
    /// No ECDSA keypair found in the keystore.
    #[error("No ECDSA keypair found in the keystore")]
    NoEcdsaKeypair,
    /// Invalid ECDSA keypair found in the keystore.
    #[error("Invalid ECDSA keypair found in the keystore")]
    InvalidEcdsaKeypair,
    /// Test setup error
    #[error("Test setup error: {0}")]
    TestSetup(String),
    /// Missing `EigenlayerContractAddresses`
    #[error("Missing EigenlayerContractAddresses")]
    MissingEigenlayerContractAddresses,
    /// Missing `SymbioticContractAddresses`
    #[error("Missing SymbioticContractAddresses")]
    MissingSymbioticContractAddresses,

    #[error("{0}")]
    Other(#[from] Box<dyn core::error::Error + Send + Sync>),
}
