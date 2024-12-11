#[cfg(feature = "aws-signer")]
pub mod aws;
#[cfg(feature = "gcp-signer")]
pub mod gcp;
#[cfg(any(feature = "ledger-browser", feature = "ledger-node"))]
pub mod ledger;

use crate::error::Result;
use crate::key_types::KeyType;
use async_trait::async_trait;
use gadget_std::future::Future;
use serde::{de::DeserializeOwned, Serialize};

// Configuration for different remote systems
#[derive(Clone, Debug)]
pub enum RemoteConfig {
    #[cfg(feature = "aws-signer")]
    Aws { keys: Vec<aws::AwsKeyConfig> },
    #[cfg(feature = "gcp-signer")]
    Gcp { keys: Vec<gcp::GcpKeyConfig> },
    #[cfg(any(feature = "ledger-browser", feature = "ledger-node"))]
    Ledger { keys: Vec<ledger::LedgerKeyConfig> },
}

/// Capabilities that a remote backend can support
#[derive(Debug, Clone, Default)]
pub struct RemoteCapabilities {
    pub signing: bool,
    pub key_generation: bool,
    pub key_derivation: bool,
    pub encryption: bool,
}

/// Core trait for remote key operations
pub trait RemoteBackend: Send + Sync {
    /// Get the capabilities of this backend
    fn capabilities(&self) -> RemoteCapabilities;

    /// Get the supported key types
    fn supported_key_types(&self) -> Vec<&'static str>;
}

/// Trait for remote signing operations
pub trait RemoteSigner<T: KeyType>: RemoteBackend {
    /// Get the public key
    fn get_public_key(&self) -> impl Future<Output = Result<T::Public>> + Send;

    /// Sign a message
    fn sign(&self, msg: &[u8]) -> impl Future<Output = Result<T::Signature>> + Send;

    /// Sign a pre-hashed message
    fn sign_prehashed(&self, msg: &[u8; 32]) -> impl Future<Output = Result<T::Signature>> + Send;
}

// Keep existing EcdsaRemoteSigner for ECDSA-specific remote signing
#[async_trait::async_trait]
pub trait EcdsaRemoteSigner<T: KeyType>: Send + Sync {
    type Public: Clone
        + Ord
        + Serialize
        + DeserializeOwned
        + std::fmt::Debug
        + From<T::Public>
        + Send;
    type Signature: Clone + Serialize + DeserializeOwned + std::fmt::Debug;
    type KeyId: Clone + Serialize + DeserializeOwned + std::fmt::Debug + Send;
    type Config: Clone + Serialize + DeserializeOwned + std::fmt::Debug;

    async fn build(config: RemoteConfig) -> Result<Self>
    where
        Self: Sized;
    async fn get_public_key(
        &self,
        key_id: &Self::KeyId,
        chain_id: Option<u64>,
    ) -> Result<Self::Public>;
    async fn iter_public_keys(&self, chain_id: Option<u64>) -> Result<Vec<Self::Public>>;
    async fn get_key_id_from_public_key(
        &self,
        public_key: &Self::Public,
        chain_id: Option<u64>,
    ) -> Result<Self::KeyId>;
    async fn sign_message_with_key_id(
        &self,
        message: &[u8],
        key_id: &Self::KeyId,
        chain_id: Option<u64>,
    ) -> Result<Self::Signature>;
}

// Generic remote operations trait for extensibility
#[async_trait]
pub trait RemoteOperations: Send + Sync {
    /// Get the type of remote system
    fn backend_type(&self) -> &'static str;

    /// Check if specific operations are supported
    fn supports_operation(&self, operation: &str) -> bool;

    /// Execute a remote operation
    async fn execute(&self, operation: &str, params: &[u8]) -> Result<Vec<u8>>;
}
