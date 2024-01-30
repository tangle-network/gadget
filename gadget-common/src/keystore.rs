use async_trait::async_trait;
use parking_lot::RwLock;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sp_core::ecdsa::Pair as EcdsaPair;
use sp_core::sr25519::Pair as Sr25519Pair;
use sp_core::{keccak_256, Pair};
use std::collections::HashMap;
use std::sync::Arc;
use tangle_primitives::jobs::JobId;

pub type ECDSAKeyStore<BE> = GenericKeyStore<BE, EcdsaPair>;
pub type Sr25519KeyStore<BE> = GenericKeyStore<BE, Sr25519Pair>;

#[derive(Clone)]
pub struct GenericKeyStore<BE: KeystoreBackend, P: Pair> {
    backend: BE,
    pair: P,
}

impl<P: Pair> GenericKeyStore<InMemoryBackend, P> {
    pub fn in_memory(pair: P) -> Self {
        GenericKeyStore {
            backend: InMemoryBackend::new(),
            pair,
        }
    }
}

impl<P: Pair, BE: KeystoreBackend> GenericKeyStore<BE, P> {
    pub fn pair(&self) -> &P {
        &self.pair
    }
}

impl<P: Pair, BE: KeystoreBackend> GenericKeyStore<BE, P> {
    pub async fn get<T: DeserializeOwned>(
        &self,
        key: &[u8; 32],
    ) -> Result<Option<T>, crate::Error> {
        self.backend.get(key).await
    }

    pub async fn get_job_result<T: DeserializeOwned>(
        &self,
        job_id: JobId,
    ) -> Result<Option<T>, crate::Error> {
        let key = keccak_256(&job_id.to_be_bytes());
        self.get(&key).await
    }

    pub async fn set<T: Serialize + Send>(
        &self,
        key: &[u8; 32],
        value: T,
    ) -> Result<(), crate::Error> {
        self.backend.set(key, value).await
    }

    pub async fn set_job_result<T: Serialize + Send>(
        &self,
        job_id: JobId,
        value: T,
    ) -> Result<(), crate::Error> {
        let key = keccak_256(&job_id.to_be_bytes());
        self.set(&key, value).await
    }
}

#[async_trait]
pub trait KeystoreBackend: Clone + Send + Sync + 'static {
    async fn get<T: DeserializeOwned>(&self, key: &[u8; 32]) -> Result<Option<T>, crate::Error>;
    async fn set<T: Serialize + Send>(&self, key: &[u8; 32], value: T) -> Result<(), crate::Error>;
}

#[derive(Clone)]
pub struct InMemoryBackend {
    map: Arc<RwLock<HashMap<[u8; 32], Vec<u8>>>>,
}

impl Default for InMemoryBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryBackend {
    pub fn new() -> Self {
        Self {
            map: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl KeystoreBackend for InMemoryBackend {
    async fn get<T: DeserializeOwned>(&self, key: &[u8; 32]) -> Result<Option<T>, crate::Error> {
        if let Some(bytes) = self.map.read().get(key).cloned() {
            let value: T =
                bincode2::deserialize(&bytes).map_err(|rr| crate::Error::KeystoreError {
                    err: format!("Failed to deserialize value: {:?}", rr),
                })?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    async fn set<T: Serialize + Send>(&self, key: &[u8; 32], value: T) -> Result<(), crate::Error> {
        let serialized = bincode2::serialize(&value).map_err(|rr| crate::Error::KeystoreError {
            err: format!("Failed to serialize value: {:?}", rr),
        })?;
        self.map.write().insert(*key, serialized);
        Ok(())
    }
}
