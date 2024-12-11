use super::StorageEntry;
use crate::error::Error;
use crate::key_types::KeyType;
use crate::keystore::remote::RemoteConfig;
use crate::storage::RawStorage;
use gadget_std::{boxed::Box, vec::Vec};
use serde::de::DeserializeOwned;

/// Backend configuration for different storage types
pub enum BackendConfig {
    /// Local storage backend
    Local(Box<dyn RawStorage>),

    /// Remote signer backend
    #[cfg(feature = "remote")]
    Remote(RemoteConfig),
}

/// Core trait for keystore backend operations
pub trait Backend: Send + Sync {
    /// Register a storage backend for a key type with priority
    fn register_storage<T: KeyType>(
        &mut self,
        config: BackendConfig,
        priority: u8,
    ) -> Result<(), Error>;

    /// Generate a new key pair
    fn generate<T: KeyType>(&self, seed: Option<&[u8]>) -> Result<T::Public, Error>
    where
        T::Public: DeserializeOwned,
        T::Secret: DeserializeOwned;

    /// Generate a key pair from a string seed
    fn generate_from_string<T: KeyType>(&self, seed_str: &str) -> Result<T::Public, Error>
    where
        T::Public: DeserializeOwned,
        T::Secret: DeserializeOwned;

    /// Sign a message using a local key
    fn sign_with_local<T: KeyType>(
        &self,
        public: &T::Public,
        msg: &[u8],
    ) -> Result<T::Signature, Error>
    where
        T::Public: DeserializeOwned,
        T::Secret: DeserializeOwned;

    /// List all public keys of a given type from local storage
    fn list_local<T: KeyType>(&self) -> Result<Vec<T::Public>, Error>
    where
        T::Public: DeserializeOwned;

    /// Get a public key from either local
    fn get_public_key_local<T: KeyType>(&self, key_id: &str) -> Result<T::Public, Error>
    where
        T::Public: DeserializeOwned;

    /// Check if a key exists in either local
    fn contains_local<T: KeyType>(&self, public: &T::Public) -> Result<bool, Error>;

    /// Remove a key from local storage
    fn remove<T: KeyType>(&self, public: &T::Public) -> Result<(), Error>
    where
        T::Public: DeserializeOwned;

    /// Get a secret key from local storage
    fn get_secret<T: KeyType>(&self, public: &T::Public) -> Result<T::Secret, Error>
    where
        T::Public: DeserializeOwned,
        T::Secret: DeserializeOwned;

    /// Get storage backends for a key type
    fn get_storage_backends<T: KeyType>(&self) -> Result<&[StorageEntry], Error>;
}
