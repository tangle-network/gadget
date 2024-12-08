use super::KeyStorage;
use crate::backend::{Error, KeyType};
use std::path::{Path, PathBuf};

pub struct FileStorage {
    root: PathBuf,
}

impl FileStorage {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let root = path.as_ref().to_path_buf();
        std::fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    fn key_path<T: KeyType>(&self, public: &T::Public) -> PathBuf
    where
        T::Public: Ord + 'static,
    {
        let type_name = std::any::type_name::<T>();
        let key_hash = format!("{:x}", blake3::hash(&format!("{:?}", public).as_bytes()));
        self.root.join(type_name).join(key_hash)
    }
}

impl KeyStorage for FileStorage {
    fn store<T: KeyType>(&self, public: &T::Public, secret: &T::Secret) -> Result<(), Error>
    where
        T::Public: Ord + 'static,
    {
        let path = self.key_path::<T>(public);
        std::fs::create_dir_all(path.parent().unwrap())?;

        // Serialize and encrypt the secret key
        let data = bincode::serialize(secret)?;
        // TODO: Add encryption
        std::fs::write(path, data)?;
        Ok(())
    }

    fn load<T: KeyType>(&self, public: &T::Public) -> Result<Option<T::Secret>, Error>
    where
        T::Public: Ord + 'static,
    {
        let path = self.key_path::<T>(public);
        if !path.exists() {
            return Ok(None);
        }

        // Read and decrypt the secret key
        let data = std::fs::read(path)?;
        // TODO: Add decryption
        let secret = bincode::deserialize(&data)?;
        Ok(Some(secret))
    }

    // ... implement other methods
}
