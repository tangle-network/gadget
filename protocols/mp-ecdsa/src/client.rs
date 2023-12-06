use crate::keystore::{ECDSAKeyStore, KeystoreBackend};
use crate::util::DebugLogger;
use crate::MpEcdsaProtocolConfig;
use pallet_jobs_rpc_runtime_api::{JobsApi, RpcResponseJobsData};
use sc_client_api::{BlockchainEvents, FinalityNotifications, HeaderBackend, ImportNotifications};
use sp_api::ProvideRuntimeApi;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;
use webb_gadget::{Backend, Block, FinalityNotification};

pub struct MpEcdsaClient<B, BE, KBE, C> {
    client: Arc<C>,
    pub(crate) key_store: ECDSAKeyStore<KBE>,
    finality_notification_stream: Arc<Mutex<FinalityNotifications<B>>>,
    block_import_notification_stream: Arc<Mutex<ImportNotifications<B>>>,
    latest_finality_notification: Arc<parking_lot::Mutex<Option<FinalityNotification<B>>>>,
    logger: DebugLogger,
    _block: std::marker::PhantomData<BE>,
}

pub async fn create_client<B: Block, BE: Backend<B>, KBE: KeystoreBackend>(
    _config: &MpEcdsaProtocolConfig,
) -> Result<MpEcdsaClient<B, BE, KBE, ()>, Box<dyn Error>> {
    Ok(MpEcdsaClient {
        _block: std::marker::PhantomData,
    })
}

pub type AccountId = u64;

pub trait ClientWithApi<B, BE>:
    BlockchainEvents<B> + HeaderBackend<B> + ProvideRuntimeApi<B> + Send + Sync
where
    B: Block,
    BE: Backend<B>,
    <Self as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
}

impl<B, BE, T> ClientWithApi<B, BE> for T
where
    B: Block,
    BE: Backend<B>,
    T: BlockchainEvents<B> + HeaderBackend<B> + ProvideRuntimeApi<B> + Send + Sync,
    <T as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
}

impl<B: Block, BE: Backend<B>, KBE: KeystoreBackend, C: ClientWithApi<B, BE>>
    MpEcdsaClient<B, BE, KBE, C>
{
    pub async fn query_jobs_by_validator(
        &self,
        validator: AccountId,
    ) -> Result<Vec<RpcResponseJobsData<AccountId>>, webb_gadget::Error> {
        exec_client_function(&self.client, |client| {
            client.runtime_api().query_jobs_by_validator(validator)
        })
        .await
        .map_err(|err| webb_gadget::Error::ClientError {
            err: format!("Failed to query jobs by validator: {err:?}"),
        })
    }
}

async fn exec_client_function<C, F, T>(client: &Arc<C>, function: F) -> T
where
    for<'a> F: FnOnce(&'a C) -> T,
    C: Send + 'static,
    T: Send + 'static,
    F: Send + 'static,
{
    let client = client.clone();
    tokio::task::spawn_blocking(move || function(&client))
        .await
        .expect("Failed to spawn blocking task")
}
