pub use crate::remote::types::{EcdsaRemoteSigner, RemoteCapabilities, RemoteConfig};

use super::backend::Backend;

#[derive(Clone)]
/// Represents a remote signer configuration and its capabilities
pub struct RemoteEntry {
    config: RemoteConfig,
    capabilities: RemoteCapabilities,
}

impl RemoteEntry {
    /// Create a new remote signer entry
    pub fn new(config: RemoteConfig, capabilities: RemoteCapabilities) -> Self {
        Self {
            config,
            capabilities,
        }
    }

    /// Get the remote signer configuration
    pub fn config(&self) -> &RemoteConfig {
        &self.config
    }

    /// Get the remote signer capabilities
    pub fn capabilities(&self) -> &RemoteCapabilities {
        &self.capabilities
    }
}

pub trait RemoteBackend: Backend {
    /// Sign a message using a remote signer
    #[cfg(feature = "remote-signing")]
    async fn sign_with_remote<T: KeyType>(
        &self,
        public: &T::Public,
        msg: &[u8],
        chain_id: Option<u64>,
    ) -> Result<T::Signature, Error>
    where
        T::Public: DeserializeOwned;

    /// List all public keys from remote signers
    #[cfg(feature = "remote-signing")]
    async fn list_remote<T: KeyType>(&self, chain_id: Option<u64>) -> Result<Vec<T::Public>, Error>
    where
        T::Public: DeserializeOwned;
}
