#[cfg(feature = "bn254")]
pub mod bn254;
#[cfg(feature = "eigenlayer")]
pub mod eigenlayer;
#[cfg(feature = "evm")]
pub mod evm;

cfg_remote! {
    pub mod remote;
}

#[cfg(feature = "tangle")]
pub mod tangle;

use super::LocalStorageEntry;
use crate::error::Result;
use crate::storage::RawStorage;
use gadget_crypto::IntoCryptoError;
use gadget_crypto::KeyType;
use gadget_std::{boxed::Box, vec::Vec};
use serde::de::DeserializeOwned;

/// Backend configuration for different storage types
pub enum BackendConfig {
    /// Local storage backend
    Local(Box<dyn RawStorage>),

    /// Remote signer backend
    #[cfg(feature = "remote")]
    Remote(remote::RemoteConfig),
}

/// Core trait for keystore backend operations
pub trait Backend: Send + Sync {
    /// Generate a new key pair
    fn generate<T: KeyType>(&self, seed: Option<&[u8]>) -> Result<T::Public>
    where
        T::Public: DeserializeOwned,
        T::Secret: DeserializeOwned,
        T::Error: IntoCryptoError;

    /// Generate a key pair from a string seed
    fn generate_from_string<T: KeyType>(&self, seed_str: &str) -> Result<T::Public>
    where
        T::Public: DeserializeOwned,
        T::Secret: DeserializeOwned,
        T::Error: IntoCryptoError;

    /// Sign a message using a local key
    fn sign_with_local<T: KeyType>(&self, public: &T::Public, msg: &[u8]) -> Result<T::Signature>
    where
        T::Public: DeserializeOwned,
        T::Secret: DeserializeOwned,
        T::Error: IntoCryptoError;

    /// List all public keys of a given type from local storage
    fn list_local<T: KeyType>(&self) -> Result<Vec<T::Public>>
    where
        T::Public: DeserializeOwned;

    /// Get whichever key of the given type that occurs first in local storage
    fn first_local<T: KeyType>(&self) -> Result<T::Public>
    where
        T::Public: DeserializeOwned;

    /// Get a public key from either local
    fn get_public_key_local<T: KeyType>(&self, key_id: &str) -> Result<T::Public>
    where
        T::Public: DeserializeOwned;

    /// Check if a key exists in either local
    fn contains_local<T: KeyType>(&self, public: &T::Public) -> Result<bool>;

    /// Remove a key from local storage
    fn remove<T: KeyType>(&self, public: &T::Public) -> Result<()>
    where
        T::Public: DeserializeOwned;

    /// Get a secret key from local storage
    fn get_secret<T: KeyType>(&self, public: &T::Public) -> Result<T::Secret>
    where
        T::Public: DeserializeOwned,
        T::Secret: DeserializeOwned;

    /// Get storage backends for a key type
    fn get_storage_backends<T: KeyType>(&self) -> Result<&[LocalStorageEntry]>;
}
