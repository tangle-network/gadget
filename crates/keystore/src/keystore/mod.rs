pub mod backends;
use backends::Backend;
use backends::BackendConfig;
cfg_remote! {
    use backends::remote::RemoteEntry;
}

mod config;
pub use config::KeystoreConfig;
use gadget_crypto::KeyType;
use gadget_crypto::KeyTypeId;
use gadget_crypto::{IntoCryptoError, KeyEncoding};

use crate::error::{Error, Result};
#[cfg(feature = "std")]
use crate::storage::FileStorage;
use crate::storage::{InMemoryStorage, RawStorage};
use gadget_std::{boxed::Box, cmp, collections::BTreeMap, vec::Vec};
use serde::de::DeserializeOwned;

/// Represents a storage backend with its priority
pub struct LocalStorageEntry {
    storage: Box<dyn RawStorage>,
    priority: u8,
}

pub struct Keystore {
    storages: BTreeMap<KeyTypeId, Vec<LocalStorageEntry>>,
    #[cfg(any(
        feature = "aws-signer",
        feature = "gcp-signer",
        feature = "ledger-browser",
        feature = "ledger-node"
    ))]
    remotes: BTreeMap<KeyTypeId, Vec<RemoteEntry>>,
}

impl Keystore {
    /// Create a new `Keystore`
    ///
    /// See [`KeystoreConfig`] for notes on the backing storing.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use gadget_keystore::backends::Backend;
    /// use gadget_keystore::crypto::zebra_ed25519::Ed25519Zebra;
    /// use gadget_keystore::{Keystore, KeystoreConfig};
    ///
    /// # fn main() -> gadget_keystore::Result<()> {
    /// // Create a simple in-memory keystore
    /// let config = KeystoreConfig::new().in_memory(true);
    /// let keystore = Keystore::new(config)?;
    ///
    /// // Generate a new key pair
    /// keystore.generate::<Ed25519Zebra>(None)?;
    /// # Ok(()) }
    /// ```
    pub fn new(config: KeystoreConfig) -> Result<Self> {
        let config = config.finalize();

        let mut keystore = Self {
            storages: BTreeMap::new(),
            #[cfg(any(
                feature = "aws-signer",
                feature = "gcp-signer",
                feature = "ledger-browser",
                feature = "ledger-node"
            ))]
            remotes: BTreeMap::new(),
        };

        if config.in_memory {
            for key_type in KeyTypeId::ENABLED {
                keystore.register_storage(
                    *key_type,
                    BackendConfig::Local(Box::new(InMemoryStorage::new())),
                    0,
                )?;
            }
        }

        #[cfg(feature = "std")]
        if let Some(fs_root) = config.fs_root {
            for key_type in KeyTypeId::ENABLED {
                keystore.register_storage(
                    *key_type,
                    BackendConfig::Local(Box::new(FileStorage::new(fs_root.as_path())?)),
                    0,
                )?;
            }
        }

        #[cfg(any(
            feature = "aws-signer",
            feature = "gcp-signer",
            feature = "ledger-browser",
            feature = "ledger-node"
        ))]
        for remote_config in config.remote_configs {
            for key_type in KeyTypeId::ENABLED {
                keystore.register_storage(
                    *key_type,
                    BackendConfig::Remote(remote_config.clone()),
                    0,
                )?;
            }
        }

        Ok(keystore)
    }

    /// Register a storage backend for a key type with priority
    fn register_storage(
        &mut self,
        key_type_id: KeyTypeId,
        storage: BackendConfig,
        priority: u8,
    ) -> Result<()> {
        match storage {
            BackendConfig::Local(storage) => {
                let entry = LocalStorageEntry { storage, priority };
                let backends = self.storages.entry(key_type_id).or_default();
                backends.push(entry);
                backends.sort_by_key(|e| cmp::Reverse(e.priority));
            }
            #[cfg(any(
                feature = "aws-signer",
                feature = "gcp-signer",
                feature = "ledger-browser",
                feature = "ledger-node"
            ))]
            BackendConfig::Remote(_config) => return Err(Error::StorageNotSupported),
        }
        Ok(())
    }
}

impl Backend for Keystore {
    /// Generate a new key pair from random seed
    fn generate<T: KeyType>(&self, seed: Option<&[u8]>) -> Result<T::Public>
    where
        T::Public: DeserializeOwned,
        T::Secret: DeserializeOwned,
        T::Error: IntoCryptoError,
    {
        let backends = self.get_storage_backends::<T>()?;
        let secret = T::generate_with_seed(seed).map_err(IntoCryptoError::into_crypto_error)?;
        let public = T::public_from_secret(&secret);

        // Store in all available storage backends
        for entry in backends {
            entry
                .storage
                .store_raw(T::key_type_id(), public.to_bytes(), secret.to_bytes())?;
        }

        Ok(public)
    }

    /// Generate a key pair from a string seed
    fn generate_from_string<T: KeyType>(&self, seed_str: &str) -> Result<T::Public>
    where
        T::Public: DeserializeOwned,
        T::Secret: DeserializeOwned,
        T::Error: IntoCryptoError,
    {
        let seed = blake3::hash(seed_str.as_bytes()).as_bytes().to_vec();
        self.generate::<T>(Some(&seed))
    }

    /// Sign a message using a local key
    fn sign_with_local<T: KeyType>(&self, public: &T::Public, msg: &[u8]) -> Result<T::Signature>
    where
        T::Public: DeserializeOwned,
        T::Secret: DeserializeOwned,
        T::Error: IntoCryptoError,
    {
        let secret = self.get_secret::<T>(public)?;
        Ok(T::sign_with_secret(&mut secret.clone(), msg)
            .map_err(IntoCryptoError::into_crypto_error)?)
    }

    /// List all public keys of a given type from storages
    fn list_local<T: KeyType>(&self) -> Result<Vec<T::Public>>
    where
        T::Public: DeserializeOwned,
    {
        let mut keys = Vec::new();
        let key_type = T::key_type_id();

        if let Some(backends) = self.storages.get(&key_type) {
            for entry in backends {
                let mut backend_keys: Vec<T::Public> = entry
                    .storage
                    .list_raw(T::key_type_id())
                    .filter_map(|bytes| T::Public::from_bytes(&bytes).ok())
                    .collect();
                keys.append(&mut backend_keys);
            }
        }

        keys.sort_unstable();
        keys.dedup();
        Ok(keys)
    }

    fn get_public_key_local<T: KeyType>(&self, key_id: &str) -> Result<T::Public>
    where
        T::Public: DeserializeOwned,
    {
        // First check local storage
        let storages = self
            .storages
            .get(&T::key_type_id())
            .ok_or(Error::KeyTypeNotSupported)?;

        for entry in storages {
            if let Some(bytes) = entry
                .storage
                .load_secret_raw(T::key_type_id(), key_id.into())?
            {
                let public: T::Public = T::Public::from_bytes(&bytes)?;
                return Ok(public);
            }
        }

        Err(Error::KeyNotFound)
    }

    fn contains_local<T: KeyType>(&self, public: &T::Public) -> Result<bool> {
        let public_bytes = public.to_bytes();
        let storages = self
            .storages
            .get(&T::key_type_id())
            .ok_or(Error::KeyTypeNotSupported)?;

        for entry in storages {
            if entry
                .storage
                .contains_raw(T::key_type_id(), public_bytes.clone())
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn remove<T: KeyType>(&self, public: &T::Public) -> Result<()>
    where
        T::Public: DeserializeOwned,
    {
        let public_bytes = public.to_bytes();
        let storages = self
            .storages
            .get(&T::key_type_id())
            .ok_or(Error::KeyTypeNotSupported)?;

        for entry in storages {
            entry
                .storage
                .remove_raw(T::key_type_id(), public_bytes.clone())?;
        }

        Ok(())
    }

    fn get_secret<T: KeyType>(&self, public: &T::Public) -> Result<T::Secret>
    where
        T::Public: DeserializeOwned,
        T::Secret: DeserializeOwned,
    {
        let storages = self
            .storages
            .get(&T::key_type_id())
            .ok_or(Error::KeyTypeNotSupported)?;

        let public_bytes = public.to_bytes();
        for entry in storages {
            if let Some(bytes) = entry
                .storage
                .load_secret_raw(T::key_type_id(), public_bytes.clone())?
            {
                let secret: T::Secret = T::Secret::from_bytes(&bytes)?;
                return Ok(secret);
            }
        }

        Err(Error::KeyNotFound)
    }

    // Helper methods
    fn get_storage_backends<T: KeyType>(&self) -> Result<&[LocalStorageEntry]> {
        self.storages
            .get(&T::key_type_id())
            .map(|v| v.as_slice())
            .ok_or(Error::KeyTypeNotSupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gadget_crypto::bls_crypto::bls377::W3fBls377;
    use gadget_crypto::bls_crypto::bls381::W3fBls381;
    use gadget_crypto::ed25519_crypto::Ed25519Zebra;
    use gadget_crypto::k256_crypto::K256Ecdsa;
    use gadget_crypto::sp_core_crypto::{SpBls377, SpBls381, SpEcdsa, SpSr25519};
    use gadget_crypto::sr25519_crypto::SchnorrkelSr25519;

    #[test]
    fn test_generate_from_string() -> Result<()> {
        let keystore = Keystore::new(KeystoreConfig::new())?;

        let seed = "test seed string";
        let public1 = keystore.generate_from_string::<K256Ecdsa>(seed)?;
        let public2 = keystore.generate_from_string::<K256Ecdsa>(seed)?;

        // Same seed should generate same key
        assert_eq!(public1, public2);

        // Different seeds should generate different keys
        let public3 = keystore.generate_from_string::<K256Ecdsa>("different seed")?;
        assert_ne!(public1, public3);

        Ok(())
    }

    macro_rules! local_operations {
        ($($key_ty:ty),+ $(,)?) => {
            $(
            paste::paste! {
                #[tokio::test]
                async fn [<test_local_ $key_ty:snake>]() -> Result<()> {
                    test_local_operations_inner::<$key_ty>().await
                }
            }
            )+
        }
    }

    local_operations!(
        K256Ecdsa,
        Ed25519Zebra,
        W3fBls377,
        W3fBls381,
        SchnorrkelSr25519
    );

    // sp-core backend
    local_operations!(SpBls377, SpBls381, SpEcdsa, SpSr25519,);

    async fn test_local_operations_inner<T: KeyType>() -> Result<()>
    where
        <T as gadget_crypto::KeyType>::Error: IntoCryptoError,
    {
        let keystore = Keystore::new(KeystoreConfig::new())?;

        // Generate and test local key
        let public = keystore.generate::<T>(None)?;
        let message = b"test message";
        let signature = keystore.sign_with_local::<T>(&public, message)?;
        assert!(T::verify(&public, message, &signature));

        // List local keys
        let local_keys = keystore.list_local::<T>()?;
        assert_eq!(local_keys.len(), 1);
        if local_keys[0] != public {
            panic!("Expected local key to be the same as generated key");
        }

        Ok(())
    }
}
