use crate::debug_logger::DebugLogger;
use crate::keystore::{ECDSAKeyStore, KeystoreBackend};
use async_trait::async_trait;
use auto_impl::auto_impl;
use gadget_core::gadget::substrate::Client;
use pallet_jobs_rpc_runtime_api::{BlockNumberOf, JobsApi};
use sc_client_api::{
    Backend, BlockImportNotification, BlockchainEvents, FinalityNotification,
    FinalityNotifications, ImportNotifications, StorageEventStream, StorageKey,
};
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_api::{BlockT as Block, Decode, Encode};
use sp_core::Pair;
use std::sync::Arc;
use subxt::ext::sp_core::Pair as SubxtPair;
use subxt::tx::{PairSigner, TxPayload};
use subxt::utils::H256;
use subxt::{OnlineClient, PolkadotConfig};
use tangle_primitives::jobs::{JobId, JobType};
use tangle_primitives::jobs::{PhaseResult, RpcResponseJobsData};
use tangle_primitives::roles::RoleType;

pub struct JobsClient<B: Block, BE, C> {
    pub client: C,
    logger: DebugLogger,
    pallet_tx: Arc<dyn PalletSubmitter>,
    _block: std::marker::PhantomData<(BE, B)>,
}

impl<B: Block, BE, C: Clone> Clone for JobsClient<B, BE, C> {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            logger: self.logger.clone(),
            pallet_tx: self.pallet_tx.clone(),
            _block: self._block,
        }
    }
}

pub async fn create_client<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>>(
    client: C,
    logger: DebugLogger,
    pallet_tx: Arc<dyn PalletSubmitter>,
) -> Result<JobsClient<B, BE, C>, crate::Error>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    Ok(JobsClient {
        client,
        logger,
        pallet_tx,
        _block: std::marker::PhantomData,
    })
}

pub type AccountId = sp_core::ecdsa::Public;
pub type MaxParticipants = tangle_testnet_runtime::MaxParticipants;
pub type MaxSubmissionLen = tangle_testnet_runtime::MaxSubmissionLen;
pub type MaxKeyLen = tangle_testnet_runtime::MaxKeyLen;
pub type MaxDataLen = tangle_testnet_runtime::MaxDataLen;
pub type MaxSignatureLen = tangle_testnet_runtime::MaxSignatureLen;
pub type MaxProofLen = tangle_testnet_runtime::MaxProofLen;
pub type GadgetJobResult = tangle_testnet_runtime::JobResult<
    MaxParticipants,
    MaxKeyLen,
    MaxSignatureLen,
    MaxDataLen,
    MaxProofLen,
>;
pub type GadgetJobType = JobType<AccountId, MaxParticipants, MaxSubmissionLen>;
pub type GadgetPhaseResult<B> = PhaseResult<
    AccountId,
    B,
    MaxParticipants,
    MaxKeyLen,
    MaxDataLen,
    MaxSignatureLen,
    MaxSubmissionLen,
    MaxProofLen,
>;

pub trait JobsApiForGadget<B: Block>:
    JobsApi<
    B,
    AccountId,
    MaxParticipants,
    MaxSubmissionLen,
    MaxKeyLen,
    MaxDataLen,
    MaxSignatureLen,
    MaxProofLen,
>
{
}
// Implement JobsApiForGadget for any T that implements JobsApi
impl<
        B: Block,
        T: JobsApi<
            B,
            AccountId,
            MaxParticipants,
            MaxSubmissionLen,
            MaxKeyLen,
            MaxDataLen,
            MaxSignatureLen,
            MaxProofLen,
        >,
    > JobsApiForGadget<B> for T
{
}

pub trait ClientWithApi<B, BE>:
    BlockchainEvents<B> + ProvideRuntimeApi<B> + Send + Sync + Client<B> + Clone + 'static
where
    B: Block,
    BE: Backend<B>,
    <Self as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
}

impl<B, BE, T> ClientWithApi<B, BE> for T
where
    B: Block,
    BE: Backend<B>,
    T: BlockchainEvents<B> + ProvideRuntimeApi<B> + Send + Sync + Client<B> + Clone + 'static,
    <T as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
}

impl<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>> ProvideRuntimeApi<B>
    for JobsClient<B, BE, C>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    type Api = C::Api;

    fn runtime_api(&self) -> ApiRef<Self::Api> {
        self.client.runtime_api()
    }
}

impl<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>> BlockchainEvents<B> for JobsClient<B, BE, C>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    fn import_notification_stream(&self) -> ImportNotifications<B> {
        self.client.import_notification_stream()
    }

    fn every_import_notification_stream(&self) -> ImportNotifications<B> {
        self.client.every_import_notification_stream()
    }

    fn finality_notification_stream(&self) -> FinalityNotifications<B> {
        self.client.finality_notification_stream()
    }

    fn storage_changes_notification_stream(
        &self,
        filter_keys: Option<&[StorageKey]>,
        child_filter_keys: Option<&[(StorageKey, Option<Vec<StorageKey>>)]>,
    ) -> sc_client_api::blockchain::Result<StorageEventStream<B::Hash>> {
        self.client
            .storage_changes_notification_stream(filter_keys, child_filter_keys)
    }
}

impl<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>> JobsClient<B, BE, C>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    pub async fn query_jobs_by_validator(
        &self,
        at: <B as Block>::Hash,
        validator: AccountId,
    ) -> Result<
        Vec<RpcResponseJobsData<AccountId, BlockNumberOf<B>, MaxParticipants, MaxSubmissionLen>>,
        crate::Error,
    > {
        exec_client_function(&self.client, move |client| {
            client.runtime_api().query_jobs_by_validator(at, validator)
        })
        .await
        .map_err(|err| crate::Error::ClientError {
            err: format!("Failed to query jobs by validator: {err:?}"),
        })
        .map(|r| r.unwrap_or_default())
    }

    pub async fn query_job_by_id(
        &self,
        at: <B as Block>::Hash,
        role_type: RoleType,
        job_id: JobId,
    ) -> Result<
        Option<RpcResponseJobsData<AccountId, BlockNumberOf<B>, MaxParticipants, MaxSubmissionLen>>,
        crate::Error,
    > {
        exec_client_function(&self.client, move |client| {
            client.runtime_api().query_job_by_id(at, role_type, job_id)
        })
        .await
        .map_err(|err| crate::Error::ClientError {
            err: format!("Failed to query job by id: {err:?}"),
        })
    }

    pub async fn query_job_result(
        &self,
        at: <B as Block>::Hash,
        role_type: RoleType,
        job_id: JobId,
    ) -> Result<
        Option<
            PhaseResult<
                AccountId,
                BlockNumberOf<B>,
                MaxParticipants,
                MaxKeyLen,
                MaxDataLen,
                MaxSignatureLen,
                MaxSubmissionLen,
                MaxProofLen,
            >,
        >,
        crate::Error,
    > {
        exec_client_function(&self.client, move |client| {
            client.runtime_api().query_job_result(at, role_type, job_id)
        })
        .await
        .map_err(|err| crate::Error::ClientError {
            err: format!("Failed to query phase 1 by id: {err:?}"),
        })
    }

    pub async fn submit_job_result(
        &self,
        role_type: RoleType,
        job_id: JobId,
        result: GadgetJobResult,
    ) -> Result<(), crate::Error> {
        self.pallet_tx
            .submit_job_result(role_type, job_id, result)
            .await
    }
}

#[async_trait]
impl<B, BE, C> Client<B> for JobsClient<B, BE, C>
where
    B: Block,
    BE: Backend<B>,
    C: ClientWithApi<B, BE>,
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    async fn get_next_finality_notification(&self) -> Option<FinalityNotification<B>> {
        self.client.get_next_finality_notification().await
    }

    async fn get_latest_finality_notification(&self) -> Option<FinalityNotification<B>> {
        self.client.get_latest_finality_notification().await
    }

    async fn get_next_block_import_notification(&self) -> Option<BlockImportNotification<B>> {
        self.client.get_next_block_import_notification().await
    }
}

#[async_trait]
#[auto_impl(Arc)]
pub trait PalletSubmitter: Send + Sync + 'static {
    async fn submit_job_result(
        &self,
        role_type: RoleType,
        job_id: JobId,
        result: GadgetJobResult,
    ) -> Result<(), crate::Error>;
}

pub struct SubxtPalletSubmitter {
    subxt_client: OnlineClient<PolkadotConfig>,
    subxt_pair: subxt::ext::sp_core::ecdsa::Pair,
}

#[async_trait]
impl PalletSubmitter for SubxtPalletSubmitter {
    async fn submit_job_result(
        &self,
        role_type: RoleType,
        job_id: JobId,
        result: GadgetJobResult,
    ) -> Result<(), crate::Error> {
        let tx = tangle::tx().jobs().submit_job_result(
            Decode::decode(&mut role_type.encode().as_slice()).expect("Should decode"),
            job_id,
            Decode::decode(&mut result.encode().as_slice()).expect("Should decode"),
        );
        self.submit(&tx)
            .await
            .map(|_| ())
            .map_err(|err| crate::Error::ClientError {
                err: format!("Failed to submit job result: {err:?}"),
            })
    }
}

impl SubxtPalletSubmitter {
    pub async fn new<KBE: KeystoreBackend>(
        key_store: &ECDSAKeyStore<KBE>,
    ) -> Result<Self, crate::Error> {
        let subxt_client = OnlineClient::<PolkadotConfig>::new().await.map_err(|err| {
            crate::Error::ClientError {
                err: format!("Failed to setup api: {err:?}"),
            }
        })?;

        let raw_pair = key_store.pair().to_raw_vec();
        let subxt_pair: subxt::ext::sp_core::ecdsa::Pair =
            subxt::ext::sp_core::ecdsa::Pair::from_seed_slice(&raw_pair)
                .expect("Should create pair");
        Ok(Self {
            subxt_client,
            subxt_pair,
        })
    }
    async fn submit<Call: TxPayload>(&self, call: &Call) -> anyhow::Result<H256> {
        let signer = PairSigner::new(self.subxt_pair.clone());
        Ok(self
            .subxt_client
            .tx()
            .sign_and_submit_then_watch_default(call, &signer)
            .await?
            .wait_for_finalized_success()
            .await?
            .block_hash())
    }
}

async fn exec_client_function<C: Clone, F, T>(client: &C, function: F) -> T
where
    for<'a> F: FnOnce(&'a C) -> T,
    C: Send + Sync + 'static,
    T: Send + 'static,
    F: Send + 'static,
{
    let client = client.clone();
    tokio::task::spawn_blocking(move || function(&client))
        .await
        .expect("Failed to spawn blocking task")
}

#[subxt::subxt(runtime_metadata_path = "../metadata/tangle.scale")]
pub mod tangle {}
