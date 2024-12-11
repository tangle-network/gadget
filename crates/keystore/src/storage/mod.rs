use crate::error::Result;
use crate::key_types::KeyType;
use gadget_std::{any::TypeId, boxed::Box, vec::Vec};
use serde::de::DeserializeOwned;

#[cfg(feature = "std")]
mod fs;
#[cfg(feature = "std")]
pub use fs::FileStorage;
mod in_memory;
pub use in_memory::InMemoryStorage;

// Raw storage trait that can be made into a trait object
pub trait RawStorage: Send + Sync {
    fn store_raw(
        &self,
        type_id: TypeId,
        public_bytes: Vec<u8>,
        secret_bytes: Vec<u8>,
    ) -> Result<()>;
    fn load_raw(&self, type_id: TypeId, public_bytes: Vec<u8>) -> Result<Option<Box<[u8]>>>;
    fn remove_raw(&self, type_id: TypeId, public_bytes: Vec<u8>) -> Result<()>;
    fn contains_raw(&self, type_id: TypeId, public_bytes: Vec<u8>) -> bool;
    fn list_raw(&self, type_id: TypeId) -> Box<dyn Iterator<Item = Box<[u8]>> + '_>;
}

// Type-safe wrapper around raw storage
pub struct TypedStorage<S: RawStorage> {
    storage: S,
}

impl<S: RawStorage> TypedStorage<S> {
    pub fn new(storage: S) -> Self {
        Self { storage }
    }

    pub fn store<T: KeyType>(&self, public: &T::Public, secret: &T::Secret) -> Result<()> {
        let public_bytes = serde_json::to_vec(public)?;
        let secret_bytes = serde_json::to_vec(secret)?;
        self.storage
            .store_raw(TypeId::of::<T>(), public_bytes, secret_bytes)
    }

    pub fn load<T: KeyType>(&self, public: &T::Public) -> Result<Option<T::Secret>>
    where
        T::Secret: DeserializeOwned,
    {
        let public_bytes = serde_json::to_vec(public)?;
        match self.storage.load_raw(TypeId::of::<T>(), public_bytes)? {
            Some(secret_bytes) => {
                let secret = serde_json::from_slice(&secret_bytes)?;
                Ok(Some(secret))
            }
            None => Ok(None),
        }
    }

    pub fn remove<T: KeyType>(&self, public: &T::Public) -> Result<()> {
        let public_bytes = serde_json::to_vec(public)?;
        self.storage.remove_raw(TypeId::of::<T>(), public_bytes)
    }

    pub fn contains<T: KeyType>(&self, public: &T::Public) -> bool {
        if let Ok(public_bytes) = serde_json::to_vec(public) {
            self.storage.contains_raw(TypeId::of::<T>(), public_bytes)
        } else {
            false
        }
    }

    pub fn list<T: KeyType>(&self) -> impl Iterator<Item = T::Public> + '_
    where
        T::Public: DeserializeOwned,
    {
        self.storage
            .list_raw(TypeId::of::<T>())
            .filter_map(move |bytes| serde_json::from_slice(&bytes).ok())
    }
}
