use crate::backend::{Error, KeyType};
use gadget_std::sync::Arc;

/// Trait for key storage backends
pub trait KeyStorage: Send + Sync {
    /// Store a key pair
    fn store<T: KeyType>(&self, public: &T::Public, secret: &T::Secret) -> Result<(), Error>
    where
        T::Public: Ord + 'static;

    /// Load a secret key
    fn load<T: KeyType>(&self, public: &T::Public) -> Result<Option<T::Secret>, Error>
    where
        T::Public: Ord + 'static;

    /// Remove a key pair
    fn remove<T: KeyType>(&self, public: &T::Public) -> Result<(), Error>
    where
        T::Public: Ord + 'static;

    /// Check if a key exists
    fn contains<T: KeyType>(&self, public: &T::Public) -> bool
    where
        T::Public: Ord + 'static;

    /// List all public keys of a given type
    fn list<T: KeyType>(&self) -> Box<dyn Iterator<Item = T::Public>>
    where
        T::Public: Ord + Clone + 'static;
}

/// Storage configuration for different key types
#[derive(Clone)]
pub enum StorageConfig {
    /// Store in memory
    Memory,
    /// Store in filesystem
    File(std::path::PathBuf),
    /// Remote signer
    Remote(String), // URL or connection string
}

/// Storage backend for a specific key type
#[derive(Clone)]
pub enum StorageBackend {
    Memory(Arc<InMemoryStorage>),
    File(Arc<FileStorage>),
    Remote(Arc<RemoteStorage>),
}

pub mod fs;
pub mod in_memory;
