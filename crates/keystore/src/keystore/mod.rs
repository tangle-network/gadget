use crate::backend::{Backend, Error, KeyType};
use crate::storage::fs::FileStorage;
use crate::storage::in_memory::InMemoryStorage;
use crate::storage::remote::RemoteStorage;
use crate::storage::{StorageBackend, StorageConfig};
use gadget_std::{any::TypeId, collections::btree_map::BTreeMap, sync::Arc};
use lock_api::RawRwLock;

/// A keystore implementation that manages cryptographic keys
pub struct Keystore {
    storage_config: BTreeMap<TypeId, StorageBackend>,
    default_storage: StorageBackend,
}

impl Keystore {
    /// Create a new empty keystore
    pub fn new(default_storage: StorageConfig) -> Result<Self, Error> {
        let default_storage = match default_storage {
            StorageConfig::Memory => StorageBackend::Memory(Arc::new(InMemoryStorage::new())),
            StorageConfig::File(path) => StorageBackend::File(Arc::new(FileStorage::new(path)?)),
            StorageConfig::Remote(url) => {
                StorageBackend::Remote(Arc::new(RemoteStorage::new(url)?))
            }
        };

        Ok(Self {
            storage_config: BTreeMap::new(),
            default_storage,
        })
    }

    /// Set a storage configuration for a specific key type
    pub fn set_storage<T: KeyType>(&mut self, config: StorageConfig) -> Result<(), Error> {
        let storage = match config {
            StorageConfig::Memory => StorageBackend::Memory(Arc::new(InMemoryStorage::new())),
            StorageConfig::File(path) => StorageBackend::File(Arc::new(FileStorage::new(path)?)),
            StorageConfig::Remote(url) => {
                StorageBackend::Remote(Arc::new(RemoteStorage::new(url)?))
            }
        };
        self.storage_config.insert(TypeId::of::<T>(), storage);
        Ok(())
    }

    /// Get the storage configuration for a specific key type
    fn get_storage<T: KeyType>(&self) -> &StorageBackend {
        self.storage_config
            .get(&TypeId::of::<T>())
            .unwrap_or(&self.default_storage)
    }
}

impl<R: RawRwLock> Backend for Keystore<R> {
    fn generate_new<T: KeyType>(&self, seed: Option<&[u8]>) -> Result<T::Public, Error>
    where
        T::Public: Ord + 'static,
        T::Secret: 'static,
    {
        let secret = T::generate_with_seed(seed)?;
        let public = T::public_from_secret(&secret);
        self.insert::<T>(public.clone(), secret)?;
        Ok(public)
    }

    fn sign<T: KeyType>(
        &self,
        public: &T::Public,
        msg: &[u8],
    ) -> Result<Option<T::Signature>, Error>
    where
        T::Public: Ord + 'static,
    {
        let mut secret = match self.get_secret::<T>(public)? {
            Some(s) => s,
            None => return Ok(None),
        };
        Ok(Some(T::sign_with_secret(&mut secret, msg)?))
    }

    fn expose_secret<T: KeyType>(&self, public: &T::Public) -> Result<Option<T::Secret>, Error>
    where
        T::Public: Ord + 'static,
        T::Secret: Clone,
    {
        self.get_secret::<T>(public)
    }

    fn iter_keys<T: KeyType>(&self) -> Box<dyn Iterator<Item = T::Public>>
    where
        T::Public: Ord + Clone + 'static,
    {
        self.iter_keys::<T>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key_types::{sp_ed25519::Ed25519, sp_sr25519::Sr25519};

    #[test]
    fn test_keystore_basic_operations() {
        let keystore = Keystore::<parking_lot::RawRwLock>::new();

        // Test Sr25519
        let sr_public = keystore.generate_new::<Sr25519>(None).unwrap();
        assert!(keystore.contains::<Sr25519>(&sr_public));

        let msg = b"test message";
        let sr_sig = keystore.sign::<Sr25519>(&sr_public, msg).unwrap().unwrap();
        assert!(Sr25519::verify(&sr_public, msg, &sr_sig));

        keystore.remove::<Sr25519>(&sr_public).unwrap();
        assert!(!keystore.contains::<Sr25519>(&sr_public));

        // Test Ed25519
        let ed_public = keystore.generate_new::<Ed25519>(None).unwrap();
        assert!(keystore.contains::<Ed25519>(&ed_public));

        let ed_sig = keystore.sign::<Ed25519>(&ed_public, msg).unwrap().unwrap();
        assert!(Ed25519::verify(&ed_public, msg, &ed_sig));

        keystore.remove::<Ed25519>(&ed_public).unwrap();
        assert!(!keystore.contains::<Ed25519>(&ed_public));
    }

    #[test]
    fn test_keystore_iteration() {
        let keystore = Keystore::<parking_lot::RawRwLock>::new();

        // Generate multiple keys
        let sr_keys: Vec<_> = (0..3)
            .map(|_| keystore.generate_new::<Sr25519>(None).unwrap())
            .collect();

        // Test iteration
        let stored_keys: Vec<_> = keystore.iter_keys::<Sr25519>().collect();
        assert_eq!(stored_keys.len(), 3);

        for key in sr_keys {
            assert!(stored_keys.contains(&key));
        }
    }
}
