use async_trait::async_trait;
use curv::elliptic::curves::Secp256k1;
use multi_party_ecdsa::gg_2020::state_machine::keygen::LocalKey;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tangle_primitives::jobs::JobId;

#[derive(Clone)]
pub struct ECDSAKeyStore<BE: KeystoreBackend> {
    backend: BE,
}

impl<BE: KeystoreBackend> ECDSAKeyStore<BE> {
    pub fn in_memory() -> ECDSAKeyStore<InMemoryBackend> {
        ECDSAKeyStore {
            backend: InMemoryBackend::new(),
        }
    }

    pub async fn get(
        &self,
        job_id: &JobId,
    ) -> Result<Option<LocalKey<Secp256k1>>, crate::error::Error> {
        self.backend.get(job_id).await
    }

    pub async fn set(
        &self,
        job_id: JobId,
        key: LocalKey<Secp256k1>,
    ) -> Result<(), crate::error::Error> {
        self.backend.set(job_id, key).await
    }
}

#[async_trait]
pub trait KeystoreBackend: Clone + Send + Sync {
    async fn get(&self, job_id: &JobId)
        -> Result<Option<LocalKey<Secp256k1>>, crate::error::Error>;
    async fn set(&self, job_id: JobId, key: LocalKey<Secp256k1>)
        -> Result<(), crate::error::Error>;
}

#[derive(Clone)]
pub struct InMemoryBackend {
    map: Arc<RwLock<HashMap<JobId, LocalKey<Secp256k1>>>>,
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
    async fn get(
        &self,
        job_id: &JobId,
    ) -> Result<Option<LocalKey<Secp256k1>>, crate::error::Error> {
        Ok(self.map.read().get(job_id).cloned())
    }

    async fn set(
        &self,
        job_id: JobId,
        key: LocalKey<Secp256k1>,
    ) -> Result<(), crate::error::Error> {
        self.map.write().insert(job_id, key);
        Ok(())
    }
}
