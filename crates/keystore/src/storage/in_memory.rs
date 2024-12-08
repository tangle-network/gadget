use super::KeyStorage;
use crate::backend::{Error, KeyType};
use gadget_std::collections::btree_map::BTreeMap;
use gadget_std::sync::RwLock;
use std::any::TypeId;

pub struct InMemoryStorage {
    keys: RwLock<
        BTreeMap<
            TypeId,
            BTreeMap<Box<dyn std::any::Any + Send + Sync>, Box<dyn std::any::Any + Send + Sync>>,
        >,
    >,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            keys: RwLock::new(BTreeMap::new()),
        }
    }
}

impl KeyStorage for InMemoryStorage {
    fn store<T: KeyType>(&self, public: &T::Public, secret: &T::Secret) -> Result<(), Error>
    where
        T::Public: Ord + 'static,
    {
        let mut guard = self.keys.write();
        let type_map = guard.entry(TypeId::of::<T>()).or_insert_with(BTreeMap::new);

        type_map.insert(Box::new(public.clone()), Box::new(secret.clone()));
        Ok(())
    }

    fn load<T: KeyType>(&self, public: &T::Public) -> Result<Option<T::Secret>, Error>
    where
        T::Public: Ord + 'static,
    {
        let guard = self.keys.read();
        if let Some(type_map) = guard.get(&TypeId::of::<T>()) {
            if let Some(secret) = type_map.get(&Box::new(public.clone())) {
                if let Some(secret) = secret.downcast_ref::<T::Secret>() {
                    return Ok(Some(secret.clone()));
                }
            }
        }
        Ok(None)
    }

    // ... implement other methods
}
