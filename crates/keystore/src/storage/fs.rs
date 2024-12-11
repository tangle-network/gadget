use super::RawStorage;
use crate::error::Error;
use gadget_std::any::TypeId;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub struct FileStorage {
    root: PathBuf,
}

impl FileStorage {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let root = path.as_ref().to_path_buf();
        fs::create_dir_all(&root).map_err(|e| Error::Other(e.to_string()))?;
        Ok(Self { root })
    }

    fn type_dir(&self, type_id: TypeId) -> PathBuf {
        self.root.join(format!("{:?}", type_id))
    }

    fn key_path(&self, type_id: TypeId, public_bytes: &[u8]) -> PathBuf {
        let hash = blake3::hash(public_bytes);
        self.type_dir(type_id).join(hex::encode(hash.as_bytes()))
    }
}

impl RawStorage for FileStorage {
    fn store_raw(
        &self,
        type_id: TypeId,
        public_bytes: Vec<u8>,
        secret_bytes: Vec<u8>,
    ) -> Result<(), Error> {
        let path = self.key_path(type_id, &public_bytes[..]);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| Error::Other(e.to_string()))?;
        }

        let data = (public_bytes.to_vec(), secret_bytes.to_vec());
        let encoded = serde_json::to_vec(&data).map_err(|e| Error::Other(e.to_string()))?;
        fs::write(path, encoded).map_err(|e| Error::Other(e.to_string()))?;
        Ok(())
    }

    fn load_raw(&self, type_id: TypeId, public_bytes: Vec<u8>) -> Result<Option<Box<[u8]>>, Error> {
        let path = self.key_path(type_id, &public_bytes[..]);
        if !path.exists() {
            return Ok(None);
        }

        let data = fs::read(&path).map_err(|e| Error::Other(e.to_string()))?;
        let (stored_pub, secret): (Vec<u8>, Vec<u8>) =
            serde_json::from_slice(&data).map_err(|e| Error::Other(e.to_string()))?;

        // Verify the public key matches
        if stored_pub != public_bytes {
            return Ok(None);
        }

        Ok(Some(secret.into_boxed_slice()))
    }

    fn remove_raw(&self, type_id: TypeId, public_bytes: Vec<u8>) -> Result<(), Error> {
        let path = self.key_path(type_id, &public_bytes[..]);
        if path.exists() {
            fs::remove_file(path).map_err(|e| Error::Other(e.to_string()))?;
        }
        Ok(())
    }

    fn contains_raw(&self, type_id: TypeId, public_bytes: Vec<u8>) -> bool {
        self.key_path(type_id, &public_bytes[..]).exists()
    }

    fn list_raw(&self, type_id: TypeId) -> Box<dyn Iterator<Item = Box<[u8]>> + '_> {
        let type_dir = self.type_dir(type_id);
        if !type_dir.exists() {
            return Box::new(std::iter::empty());
        }

        let iter = fs::read_dir(type_dir)
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| fs::read(entry.path()).ok())
            .filter_map(|data| {
                serde_json::from_slice::<(Vec<u8>, Vec<u8>)>(&data)
                    .ok()
                    .map(|(pub_key, _)| pub_key.into_boxed_slice())
            });

        Box::new(iter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{key_types::k256_ecdsa::K256Ecdsa, key_types::KeyType, storage::TypedStorage};
    use tempfile::tempdir;

    #[test]
    fn test_basic_operations() -> Result<(), Error> {
        let temp_dir = tempdir()?;
        let raw_storage = FileStorage::new(temp_dir.path())?;
        let storage = TypedStorage::new(raw_storage);

        // Generate a key pair
        let secret = K256Ecdsa::generate_with_seed(None)?;
        let public = K256Ecdsa::public_from_secret(&secret);

        // Test store and load
        storage.store::<K256Ecdsa>(&public, &secret)?;
        let loaded = storage.load::<K256Ecdsa>(&public)?;
        assert_eq!(loaded.as_ref(), Some(&secret));

        // Test contains
        assert!(storage.contains::<K256Ecdsa>(&public));

        // Test list
        let keys: Vec<_> = storage.list::<K256Ecdsa>().collect();
        assert_eq!(keys.len(), 1);
        assert_eq!(&keys[0], &public);

        // Test remove
        storage.remove::<K256Ecdsa>(&public)?;
        assert!(!storage.contains::<K256Ecdsa>(&public));
        assert_eq!(storage.load::<K256Ecdsa>(&public)?, None);

        Ok(())
    }

    #[test]
    fn test_multiple_key_types() -> Result<(), Error> {
        let temp_dir = tempdir()?;
        let raw_storage = FileStorage::new(temp_dir.path())?;
        let storage = TypedStorage::new(raw_storage);

        // Create keys of different types
        let k256_secret = K256Ecdsa::generate_with_seed(None)?;
        let k256_public = K256Ecdsa::public_from_secret(&k256_secret);

        // Store keys
        storage.store::<K256Ecdsa>(&k256_public, &k256_secret)?;

        // Verify isolation between types
        assert!(storage.contains::<K256Ecdsa>(&k256_public));

        // List should only show keys of the requested type
        assert_eq!(storage.list::<K256Ecdsa>().count(), 1);

        Ok(())
    }
}
