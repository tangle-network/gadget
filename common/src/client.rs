use crate::debug_logger::DebugLogger;
use crate::environments::GadgetEnvironment;
use async_trait::async_trait;
use gadget_core::gadget::general::Client;
#[cfg(feature = "substrate")]
use sp_core::Pair;
use std::sync::Arc;
#[cfg(feature = "substrate")]
use tangle_subxt::subxt::{self};

pub struct JobsClient<Env: GadgetEnvironment> {
    pub client: Arc<Env::Client>,
    logger: DebugLogger,
    pub(crate) tx_manager: Env::TransactionManager,
}

impl<Env: GadgetEnvironment> JobsClient<Env> {
    pub fn logger(&self) -> &DebugLogger {
        &self.logger
    }
}

impl<Env: GadgetEnvironment> Clone for JobsClient<Env> {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            logger: self.logger.clone(),
            tx_manager: self.tx_manager.clone(),
        }
    }
}

pub async fn create_client<Env: GadgetEnvironment>(
    client: Env::Client,
    logger: DebugLogger,
    tx_manager: Env::TransactionManager,
) -> Result<JobsClient<Env>, crate::Error> {
    Ok(JobsClient {
        client: Arc::new(client),
        logger,
        tx_manager,
    })
}

pub async fn exec_client_function<C, F, T>(client: &C, function: F) -> T
where
    for<'a> F: FnOnce(&'a C) -> T,
    C: Clone + Send + Sync + 'static,
    T: Send + 'static,
    F: Send + 'static,
{
    let client = client.clone();
    gadget_io::tokio::task::spawn_blocking(move || function(&client))
        .await
        .expect("Failed to spawn blocking task")
}

#[async_trait]
impl<Env: GadgetEnvironment> Client<Env::Event> for JobsClient<Env> {
    async fn next_event(&self) -> Option<Env::Event> {
        self.client.next_event().await
    }

    async fn latest_event(&self) -> Option<Env::Event> {
        self.client.latest_event().await
    }
}

/// A [`Signer`] implementation that can be constructed from an [`sp_core::Pair`].
#[cfg(feature = "substrate")]
#[derive(Clone)]
pub struct PairSigner<T: subxt::Config> {
    account_id: T::AccountId,
    signer: sp_core::sr25519::Pair,
}

#[cfg(feature = "substrate")]
impl<T: subxt::Config> PairSigner<T>
where
    T::AccountId: From<[u8; 32]>,
{
    pub fn new(signer: sp_core::sr25519::Pair) -> Self {
        let account_id = T::AccountId::from(signer.public().into());
        Self { account_id, signer }
    }
}

#[cfg(feature = "substrate")]
impl<T: subxt::Config> subxt::tx::Signer<T> for PairSigner<T>
where
    T::Signature: From<subxt::utils::MultiSignature>,
{
    fn account_id(&self) -> T::AccountId {
        self.account_id.clone()
    }

    fn address(&self) -> T::Address {
        self.account_id.clone().into()
    }

    fn sign(&self, signer_payload: &[u8]) -> T::Signature {
        subxt::utils::MultiSignature::Sr25519(self.signer.sign(signer_payload).0).into()
    }
}
