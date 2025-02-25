//! Backing storage implementations for local keystores

use crate::error::Result;
use gadget_crypto::{BytesEncoding, KeyType, KeyTypeId};
use gadget_std::{boxed::Box, vec::Vec};
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
        type_id: KeyTypeId,
        public_bytes: Vec<u8>,
        secret_bytes: Vec<u8>,
    ) -> Result<()>;
    fn load_secret_raw(
        &self,
        type_id: KeyTypeId,
        public_bytes: Vec<u8>,
    ) -> Result<Option<Box<[u8]>>>;
    fn remove_raw(&self, type_id: KeyTypeId, public_bytes: Vec<u8>) -> Result<()>;
    fn contains_raw(&self, type_id: KeyTypeId, public_bytes: Vec<u8>) -> bool;
    fn list_raw(&self, type_id: KeyTypeId) -> Box<dyn Iterator<Item = Box<[u8]>> + '_>;
}

// Type-safe wrapper around raw storage
pub struct TypedStorage<S: RawStorage> {
    storage: S,
}

// Occurs when no features are enabled
#[cfg_attr(
    not(any(
        feature = "ecdsa",
        feature = "sr25519-schnorrkel",
        feature = "zebra",
        feature = "bls",
        feature = "bn254",
        feature = "sp-core"
    )),
    allow(unreachable_code, unused_variables, unused_mut)
)]
impl<S: RawStorage> TypedStorage<S> {
    pub fn new(storage: S) -> Self {
        Self { storage }
    }

    pub fn store<T: KeyType>(&self, public: &T::Public, secret: &T::Secret) -> Result<()> {
        let public_bytes = public.to_bytes();
        let secret_bytes = secret.to_bytes();
        self.storage
            .store_raw(T::key_type_id(), public_bytes, secret_bytes)
    }

    pub fn load<T: KeyType>(&self, public: &T::Public) -> Result<Option<T::Secret>>
    where
        T::Secret: DeserializeOwned,
    {
        let public_bytes = public.to_bytes();
        match self
            .storage
            .load_secret_raw(T::key_type_id(), public_bytes)?
        {
            Some(secret_bytes) => {
                let secret = T::Secret::from_bytes(&secret_bytes)?;
                Ok(Some(secret))
            }
            None => Ok(None),
        }
    }

    pub fn remove<T: KeyType>(&self, public: &T::Public) -> Result<()> {
        let public_bytes = public.to_bytes();
        self.storage.remove_raw(T::key_type_id(), public_bytes)
    }

    pub fn contains<T: KeyType>(&self, public: &T::Public) -> bool {
        self.storage
            .contains_raw(T::key_type_id(), public.to_bytes())
    }

    pub fn list<T: KeyType>(&self) -> impl Iterator<Item = T::Public> + '_
    where
        T::Public: DeserializeOwned,
    {
        self.storage
            .list_raw(T::key_type_id())
            .filter_map(move |bytes| T::Public::from_bytes(&bytes).ok())
    }
}
