use crate::debug_logger::DebugLogger;
use async_trait::async_trait;
use auto_impl::auto_impl;
use gadget_core::gadget::substrate::Client;
use pallet_jobs_rpc_runtime_api::{BlockNumberOf, JobsApi};
use parity_scale_codec::{Decode, Encode};
use sc_client_api::{
    Backend, BlockImportNotification, BlockchainEvents, FinalityNotification,
    FinalityNotifications, ImportNotifications, StorageEventStream, StorageKey,
};
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_core::Pair;
use sp_core::{ecdsa, ByteArray};
use sp_runtime::traits::Block;

use std::sync::Arc;
use tangle_primitives::jobs::JobId;
use tangle_primitives::jobs::{PhaseResult, RpcResponseJobsData};
use tangle_primitives::roles::RoleType;
use tangle_subxt::subxt::{self, config::ExtrinsicParams, tx::TxPayload, OnlineClient};

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

pub mod types {
    #[cfg(not(feature = "testnet"))]
    use frame_support::pallet_prelude::{Decode, Encode, TypeInfo};
    use pallet_jobs_rpc_runtime_api::JobsApi;
    #[cfg(not(feature = "testnet"))]
    use serde::{Deserialize, Serialize};
    use sp_runtime::traits::Block;
    #[cfg(not(feature = "testnet"))]
    use sp_runtime::RuntimeDebug;
    use tangle_primitives::jobs::{JobResult, JobType, PhaseResult};

    #[cfg(not(feature = "testnet"))]
    frame_support::parameter_types! {
        #[derive(Clone, RuntimeDebug, Eq, PartialEq, TypeInfo, Encode, Decode)]
        #[derive(Serialize, Deserialize)]
        pub const MaxSubmissionLen: u32 = 60_000_000;
        #[derive(Clone, RuntimeDebug, Eq, PartialEq, TypeInfo, Encode, Decode)]
        #[derive(Serialize, Deserialize)]
        pub const MaxParticipants: u32 = 10;
        #[derive(Clone, RuntimeDebug, Eq, PartialEq, TypeInfo, Encode, Decode)]
        #[derive(Serialize, Deserialize)]
        pub const MaxKeyLen: u32 = 256;
        #[derive(Clone, RuntimeDebug, Eq, PartialEq, TypeInfo, Encode, Decode)]
        #[derive(Serialize, Deserialize)]
        pub const MaxDataLen: u32 = 256;
        #[derive(Clone, RuntimeDebug, Eq, PartialEq, TypeInfo, Encode, Decode)]
        #[derive(Serialize, Deserialize)]
        pub const MaxSignatureLen: u32 = 256;
        #[derive(Clone, RuntimeDebug, Eq, PartialEq, TypeInfo, Encode, Decode)]
        #[derive(Serialize, Deserialize)]
        pub const MaxProofLen: u32 = 256;
        #[derive(Clone, RuntimeDebug, Eq, PartialEq, TypeInfo, Encode, Decode)]
        #[derive(Serialize, Deserialize)]
        pub const MaxActiveJobsPerValidator: u32 = 100;
        #[derive(Clone, RuntimeDebug, Eq, PartialEq, TypeInfo, Encode, Decode)]
        #[derive(Serialize, Deserialize)]
        pub const MaxRolesPerValidator: u32 = 100;
    }

    #[cfg(feature = "testnet")]
    pub use tangle_primitives::jobs::{
        MaxActiveJobsPerValidator, MaxDataLen, MaxKeyLen, MaxParticipants, MaxProofLen,
        MaxSignatureLen, MaxSubmissionLen,
    };
    pub use tangle_primitives::AccountId;

    pub type GadgetJobResult =
        JobResult<MaxParticipants, MaxKeyLen, MaxSignatureLen, MaxDataLen, MaxProofLen>;
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
}

pub use types::*;

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

    pub async fn query_restaker_role_key(
        &self,
        at: <B as Block>::Hash,
        address: AccountId,
    ) -> Result<Option<ecdsa::Public>, crate::Error> {
        exec_client_function(&self.client, move |client| {
            client.runtime_api().query_restaker_role_key(at, address)
        })
        .await
        .map_err(|err| crate::Error::ClientError {
            err: format!("Failed to query restaker role key: {err:?}"),
        })
        .map(|r| r.and_then(|v| ecdsa::Public::from_slice(v.as_slice()).ok()))
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
/// A [`Signer`] implementation that can be constructed from an [`sp_core::Pair`].
#[derive(Clone)]
pub struct PairSigner<T: subxt::Config> {
    account_id: T::AccountId,
    signer: sp_core::sr25519::Pair,
}

impl<T: subxt::Config> PairSigner<T>
where
    T::AccountId: From<[u8; 32]>,
{
    pub fn new(signer: sp_core::sr25519::Pair) -> Self {
        let account_id = T::AccountId::from(signer.public().into());
        Self { account_id, signer }
    }
}

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

pub struct SubxtPalletSubmitter<C, S>
where
    C: subxt::Config,
    S: subxt::tx::Signer<C>,
{
    subxt_client: OnlineClient<C>,
    signer: S,
    logger: DebugLogger,
}

#[async_trait]
impl<C, S> PalletSubmitter for SubxtPalletSubmitter<C, S>
where
    C: subxt::Config + Send + Sync + 'static,
    S: subxt::tx::Signer<C> + Send + Sync + 'static,
    C::AccountId: std::fmt::Display + Send + Sync + 'static,
    C::Hash: std::fmt::Display,
    <C as subxt::Config>::Hash: subxt::Config,
    <C as subxt::Config>::ExtrinsicParams:
        subxt::config::ExtrinsicParams<<C as subxt::Config>::Hash>,
    <<C as subxt::Config>::ExtrinsicParams as subxt::config::ExtrinsicParams<C>>::OtherParams:
        Default + Send + Sync + 'static,
    <C::ExtrinsicParams as ExtrinsicParams<C::Hash>>::OtherParams: Default + Send + Sync + 'static,
{
    async fn submit_job_result(
        &self,
        role_type: RoleType,
        job_id: JobId,
        result: GadgetJobResult,
    ) -> Result<(), crate::Error> {
        let tx = tangle_subxt::tangle_runtime::api::tx()
            .jobs()
            .submit_job_result(
                Decode::decode(&mut role_type.encode().as_slice()).expect("Should decode"),
                job_id,
                Decode::decode(&mut result.encode().as_slice()).expect("Should decode"),
            );
        match self.submit(&tx).await {
            Ok(hash) => {
                self.logger.info(format!(
                    "({}) Job result submitted for job_id: {job_id} at block: {hash}",
                    self.signer.account_id(),
                ));
                Ok(())
            }
            Err(err) if err.to_string().contains("JobNotFound") => {
                self.logger.warn(format!(
                    "({}) Job not found for job_id: {job_id}",
                    self.signer.account_id(),
                ));
                Ok(())
            }
            Err(err) => {
                return Err(crate::Error::ClientError {
                    err: format!("Failed to submit job result: {err:?}"),
                })
            }
        }
    }
}

impl<C, S> SubxtPalletSubmitter<C, S>
where
    C: subxt::Config,
    C::AccountId: std::fmt::Display,
    S: subxt::tx::Signer<C>,
    <C as subxt::Config>::Hash: subxt::Config,
    <C as subxt::Config>::ExtrinsicParams:
        subxt::config::ExtrinsicParams<<C as subxt::Config>::Hash>,
    <<C as subxt::Config>::ExtrinsicParams as subxt::config::ExtrinsicParams<C>>::OtherParams:
        std::default::Default,
    <C::ExtrinsicParams as ExtrinsicParams<C::Hash>>::OtherParams: Default,
{
    pub async fn new(signer: S, logger: DebugLogger) -> Result<Self, crate::Error> {
        let subxt_client =
            OnlineClient::<C>::new()
                .await
                .map_err(|err| crate::Error::ClientError {
                    err: format!("Failed to setup api: {err:?}"),
                })?;
        Ok(Self {
            subxt_client,
            signer,
            logger,
        })
    }
    async fn submit<Call: TxPayload>(&self, call: &Call) -> anyhow::Result<C::Hash> {
        if let Some(details) = call.validation_details() {
            self.logger.trace(format!(
                "({}) Submitting {}.{}",
                self.signer.account_id(),
                details.pallet_name,
                details.call_name
            ));
        }
        Ok(self
            .subxt_client
            .tx()
            .sign_and_submit_then_watch_default(call, &self.signer)
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

#[cfg(test)]
mod tests {

    use tangle_subxt::{
        subxt::{tx::Signer, utils::AccountId32, PolkadotConfig},
        tangle_runtime::api::runtime_types::{
            bounded_collections::bounded_vec::BoundedVec,
            tangle_primitives::{jobs, roles},
        },
    };

    use super::*;

    #[tokio::test]
    #[ignore = "This test requires a running substrate node"]
    async fn subxt_pallet_submitter() -> anyhow::Result<()> {
        let logger = DebugLogger {
            peer_id: "test".into(),
        };
        let alice = subxt_signer::sr25519::dev::alice();
        let bob = subxt_signer::sr25519::dev::bob();
        let alice_account_id =
            <subxt_signer::sr25519::Keypair as Signer<PolkadotConfig>>::account_id(&alice);
        let bob_account_id =
            <subxt_signer::sr25519::Keypair as Signer<PolkadotConfig>>::account_id(&bob);
        let pallet_tx =
            SubxtPalletSubmitter::<PolkadotConfig, _>::new(alice.clone(), logger).await?;
        let dkg_phase_one = jobs::JobSubmission {
            expiry: 100u64,
            ttl: 100u64,
            job_type: jobs::JobType::DKGTSSPhaseOne(jobs::tss::DKGTSSPhaseOneJobType {
                participants: BoundedVec::<AccountId32>(vec![alice_account_id, bob_account_id]),
                threshold: 1u8,
                permitted_caller: None,
                role_type: roles::tss::ThresholdSignatureRoleType::DfnsCGGMP21Secp256k1,
                __subxt_unused_type_params: std::marker::PhantomData,
            }),
        };
        let tx = tangle_subxt::tangle_runtime::api::tx()
            .jobs()
            .submit_job(dkg_phase_one);
        let _hash = pallet_tx.submit(&tx).await?;
        Ok(())
    }
}
