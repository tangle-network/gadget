use crate::keystore::{ECDSAKeyStore, KeystoreBackend};
use crate::util::DebugLogger;
use crate::MpEcdsaProtocolConfig;
use gadget_core::job_manager::WorkManagerInterface;
use pallet_jobs_rpc_runtime_api::{JobsApi, RpcResponseJobsData};
use sc_client_api::{Backend, BlockchainEvents, HeaderBackend};
use sp_api::BlockT as Block;
use sp_api::ProvideRuntimeApi;
use std::error::Error;
use std::sync::Arc;
use webb_gadget::gadget::work_manager::WebbWorkManager;

pub struct MpEcdsaClient<B: Block, BE, KBE: KeystoreBackend, C> {
    client: Arc<C>,
    pub(crate) key_store: ECDSAKeyStore<KBE>,
    logger: DebugLogger,
    account_id: AccountId,
    _block: std::marker::PhantomData<(BE, B)>,
}

pub async fn create_client<
    B: Block,
    BE: Backend<B>,
    KBE: KeystoreBackend,
    C: ClientWithApi<B, BE>,
>(
    config: &MpEcdsaProtocolConfig,
    client: Arc<C>,
    logger: DebugLogger,
    key_store: ECDSAKeyStore<KBE>,
) -> Result<MpEcdsaClient<B, BE, KBE, C>, Box<dyn Error>>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    Ok(MpEcdsaClient {
        client,
        key_store,
        logger,
        account_id: config.account_id,
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
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
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
        })?
        .map_err(|err| webb_gadget::Error::ClientError {
            err: format!("Failed to query jobs by validator: {err:?}"),
        })
    }

    // TODO: Make this use the proper API
    pub async fn submit_job_result(
        &self,
        job_id: <WebbWorkManager as WorkManagerInterface>::TaskID,
        result: Vec<u8>,
    ) -> Result<(), webb_gadget::Error> {
        exec_client_function(&self.client, |client| {
            client.runtime_api().submit_job_result(job_id, result)
        })
        .await
        .map_err(|err| webb_gadget::Error::ClientError {
            err: format!("Failed to submit job result: {err:?}"),
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
