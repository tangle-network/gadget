use serde::de::DeserializeOwned;

use crate::error::Error;
use crate::key_types::KeyType;
pub use crate::remote::types::{EcdsaRemoteSigner, RemoteCapabilities, RemoteConfig};

use super::{backend::Backend, Keystore};

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

#[async_trait::async_trait]
pub trait RemoteBackend: Backend {
    /// Sign a message using a remote signer
    async fn sign_with_remote<T: KeyType, R: EcdsaRemoteSigner<T>>(
        &self,
        public: &T::Public,
        msg: &[u8],
        chain_id: Option<u64>,
    ) -> Result<T::Signature, Error>
    where
        T::Public: DeserializeOwned;

    /// List all public keys from remote signers
    async fn list_remote<T: KeyType, R: EcdsaRemoteSigner<T>>(
        &self,
        chain_id: Option<u64>,
    ) -> Result<Vec<T::Public>, Error>
    where
        T::Public: DeserializeOwned;
}

#[async_trait::async_trait]
impl RemoteBackend for Keystore {
    /// Sign a message using a remote signer
    async fn sign_with_remote<T: KeyType, R: EcdsaRemoteSigner<T>>(
        &self,
        public: &T::Public,
        msg: &[u8],
        chain_id: Option<u64>,
    ) -> Result<T::Signature, Error>
    where
        T::Public: DeserializeOwned,
    {
        let remotes = self
            .remotes
            .get(&T::key_type_id())
            .ok_or(Error::KeyTypeNotSupported)?;

        for entry in remotes {
            if entry.capabilities().signing {
                let remote = R::build(entry.config().clone()).await?;
                let public_bytes = serde_json::to_vec(public)?;
                let remote_public = serde_json::from_slice::<R::Public>(&public_bytes)?;
                let key_id = remote.get_key_id_from_public_key(&remote_public).await?;
                remote.sign_message_with_key_id(msg, &key_id).await?;
            }
        }

        Err(Error::KeyNotFound)
    }

    /// List all public keys from remote signers
    async fn list_remote<T: KeyType, R: EcdsaRemoteSigner<T>>(
        &self,
        chain_id: Option<u64>,
    ) -> Result<Vec<T::Public>, Error>
    where
        T::Public: DeserializeOwned,
    {
        let mut keys = Vec::new();
        let key_type = T::key_type_id();

        if let Some(remotes) = self.remotes.get(&key_type) {
            for entry in remotes {
                if entry.capabilities().signing {
                    let remote = R::build(entry.config().clone()).await?;
                    let key_ids = remote.iter_public_keys().await?;
                    for key_id in key_ids {
                        let public_bytes = serde_json::to_vec(&key_id)?;
                        let public = serde_json::from_slice(&public_bytes)?;
                        keys.push(public);
                    }
                }
            }
        }

        Ok(keys)
    }
}
