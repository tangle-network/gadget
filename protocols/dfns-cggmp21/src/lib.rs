use crate::protocols::key_refresh::DfnsCGGMP21KeyRefreshExtraParams;
use crate::protocols::key_rotate::DfnsCGGMP21KeyRotateExtraParams;
use crate::protocols::keygen::DfnsCGGMP21KeygenExtraParams;
use crate::protocols::sign::DfnsCGGMP21SigningExtraParams;
use async_trait::async_trait;
use gadget_common::client::{AccountId, ClientWithApi, JobsClient, PalletSubmitter};
use gadget_common::client::{GadgetJobType, JobsApiForGadget};
use gadget_common::debug_logger::DebugLogger;
use gadget_common::full_protocol::SharedOptional;
use gadget_common::gadget::network::Network;
use gadget_common::gadget::{JobInitMetadata, WorkManagerConfig};
use gadget_common::keystore::{ECDSAKeyStore, KeystoreBackend};
use gadget_common::prelude::*;
use gadget_common::prelude::{FullProtocolConfig, GadgetProtocolMessage, Mutex, WorkManager};
use gadget_common::{generate_setup_and_run_command, Error};
use gadget_core::job::{BuiltExecutableJobWrapper, JobError};
use gadget_core::job_manager::{ProtocolWorkManager, WorkManagerInterface};
use protocol_macros::protocol;
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::sync::Arc;
use tangle_primitives::jobs::JobType;
use tangle_primitives::roles::{RoleType, ThresholdSignatureRoleType};
use tokio::sync::mpsc::UnboundedReceiver;

pub mod constants;
pub mod error;
pub mod protocols;
pub mod util;

#[protocol]
pub struct DfnsKeygenProtocol<
    B: Block,
    BE: Backend<B> + 'static,
    C: ClientWithApi<B, BE>,
    N: Network,
    KBE: KeystoreBackend,
> where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    pallet_tx: Arc<dyn PalletSubmitter>,
    logger: DebugLogger,
    client: C,
    network: N,
    account_id: AccountId,
    key_store: ECDSAKeyStore<KBE>,
    jobs_client: Arc<Mutex<Option<JobsClient<B, BE, C>>>>,
}

#[protocol]
pub struct DfnsSigningProtocol<
    B: Block,
    BE: Backend<B> + 'static,
    C: ClientWithApi<B, BE>,
    N: Network,
    KBE: KeystoreBackend,
> where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    pallet_tx: Arc<dyn PalletSubmitter>,
    logger: DebugLogger,
    client: C,
    network: N,
    account_id: AccountId,
    key_store: ECDSAKeyStore<KBE>,
    jobs_client: Arc<Mutex<Option<JobsClient<B, BE, C>>>>,
}

#[protocol]
pub struct DfnsKeyRefreshProtocol<
    B: Block,
    BE: Backend<B> + 'static,
    C: ClientWithApi<B, BE>,
    N: Network,
    KBE: KeystoreBackend,
> where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    pallet_tx: Arc<dyn PalletSubmitter>,
    logger: DebugLogger,
    client: C,
    network: N,
    account_id: AccountId,
    key_store: ECDSAKeyStore<KBE>,
    jobs_client: Arc<Mutex<Option<JobsClient<B, BE, C>>>>,
}

#[protocol]
pub struct DfnsKeyRotateProtocol<
    B: Block,
    BE: Backend<B> + 'static,
    C: ClientWithApi<B, BE>,
    N: Network,
    KBE: KeystoreBackend,
> where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    pallet_tx: Arc<dyn PalletSubmitter>,
    logger: DebugLogger,
    client: C,
    network: N,
    account_id: AccountId,
    key_store: ECDSAKeyStore<KBE>,
    jobs_client: Arc<Mutex<Option<JobsClient<B, BE, C>>>>,
}

#[async_trait]
impl<
        B: Block,
        BE: Backend<B> + 'static,
        C: ClientWithApi<B, BE>,
        N: Network,
        KBE: KeystoreBackend,
    > FullProtocolConfig for DfnsKeygenProtocol<B, BE, C, N, KBE>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    type AsyncProtocolParameters = DfnsCGGMP21KeygenExtraParams;
    type Client = C;
    type Block = B;
    type Backend = BE;
    type Network = N;
    type AdditionalNodeParameters = ();
    type KeystoreBackend = KBE;

    async fn new(
        client: Self::Client,
        pallet_tx: Arc<dyn PalletSubmitter>,
        network: Self::Network,
        logger: DebugLogger,
        account_id: AccountId,
        key_store: ECDSAKeyStore<Self::KeystoreBackend>,
    ) -> Result<Self, Error> {
        Ok(Self {
            pallet_tx,
            logger,
            client,
            network,
            account_id,
            key_store,
            jobs_client: Arc::new(Mutex::new(None)),
        })
    }

    async fn generate_protocol_from(
        &self,
        associated_block_id: <WorkManager as WorkManagerInterface>::Clock,
        associated_retry_id: <WorkManager as WorkManagerInterface>::RetryID,
        associated_session_id: <WorkManager as WorkManagerInterface>::SessionID,
        associated_task_id: <WorkManager as WorkManagerInterface>::TaskID,
        protocol_message_rx: UnboundedReceiver<GadgetProtocolMessage>,
        additional_params: Self::AsyncProtocolParameters,
    ) -> Result<BuiltExecutableJobWrapper, JobError> {
        protocols::keygen::generate_protocol_from(
            self,
            associated_block_id,
            associated_retry_id,
            associated_session_id,
            associated_task_id,
            protocol_message_rx,
            additional_params,
        )
        .await
    }

    fn network(&self) -> &Self::Network {
        &self.network
    }

    async fn create_next_job(
        &self,
        job: JobInitMetadata<Self::Block>,
        _work_manager: &ProtocolWorkManager<WorkManager>,
    ) -> Result<Self::AsyncProtocolParameters, Error> {
        protocols::keygen::create_next_job(self, job).await
    }

    fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    fn name(&self) -> String {
        "DFNS-Keygen-Protocol".to_string()
    }

    fn role_filter(&self, role: RoleType) -> bool {
        matches!(
            role,
            RoleType::Tss(ThresholdSignatureRoleType::DfnsCGGMP21Secp256k1)
        )
    }

    fn phase_filter(&self, job: GadgetJobType) -> bool {
        matches!(job, JobType::DKGTSSPhaseOne(_))
    }

    fn jobs_client(&self) -> &SharedOptional<JobsClient<Self::Block, Self::Backend, Self::Client>> {
        &self.jobs_client
    }

    fn pallet_tx(&self) -> Arc<dyn PalletSubmitter> {
        self.pallet_tx.clone()
    }

    fn logger(&self) -> DebugLogger {
        self.logger.clone()
    }

    fn client(&self) -> Self::Client {
        self.client.clone()
    }

    fn get_work_manager_config(&self) -> WorkManagerConfig {
        WorkManagerConfig {
            interval: None, // Manual polling
            max_active_tasks: constants::keygen_worker::MAX_RUNNING_TASKS,
            max_pending_tasks: constants::keygen_worker::MAX_ENQUEUED_TASKS,
        }
    }
}

#[async_trait]
impl<
        B: Block,
        BE: Backend<B> + 'static,
        C: ClientWithApi<B, BE>,
        N: Network,
        KBE: KeystoreBackend,
    > FullProtocolConfig for DfnsSigningProtocol<B, BE, C, N, KBE>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    type AsyncProtocolParameters = DfnsCGGMP21SigningExtraParams;
    type Client = C;
    type Block = B;
    type Backend = BE;
    type Network = N;
    type AdditionalNodeParameters = ();
    type KeystoreBackend = KBE;

    async fn new(
        client: Self::Client,
        pallet_tx: Arc<dyn PalletSubmitter>,
        network: Self::Network,
        logger: DebugLogger,
        account_id: AccountId,
        key_store: ECDSAKeyStore<Self::KeystoreBackend>,
    ) -> Result<Self, Error> {
        Ok(Self {
            pallet_tx,
            logger,
            client,
            network,
            account_id,
            key_store,
            jobs_client: Arc::new(Mutex::new(None)),
        })
    }

    async fn generate_protocol_from(
        &self,
        associated_block_id: <WorkManager as WorkManagerInterface>::Clock,
        associated_retry_id: <WorkManager as WorkManagerInterface>::RetryID,
        associated_session_id: <WorkManager as WorkManagerInterface>::SessionID,
        associated_task_id: <WorkManager as WorkManagerInterface>::TaskID,
        protocol_message_rx: UnboundedReceiver<GadgetProtocolMessage>,
        additional_params: Self::AsyncProtocolParameters,
    ) -> Result<BuiltExecutableJobWrapper, JobError> {
        protocols::sign::generate_protocol_from(
            self,
            associated_block_id,
            associated_retry_id,
            associated_session_id,
            associated_task_id,
            protocol_message_rx,
            additional_params,
        )
        .await
    }

    fn network(&self) -> &Self::Network {
        &self.network
    }

    async fn create_next_job(
        &self,
        job: JobInitMetadata<Self::Block>,
        _work_manager: &ProtocolWorkManager<WorkManager>,
    ) -> Result<Self::AsyncProtocolParameters, Error> {
        protocols::sign::create_next_job(self, job).await
    }

    fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    fn name(&self) -> String {
        "DFNS-Signing-Protocol".to_string()
    }

    fn role_filter(&self, role: RoleType) -> bool {
        matches!(
            role,
            RoleType::Tss(ThresholdSignatureRoleType::DfnsCGGMP21Secp256k1)
        )
    }

    fn phase_filter(&self, job: GadgetJobType) -> bool {
        matches!(job, JobType::DKGTSSPhaseTwo(_))
    }

    fn jobs_client(&self) -> &SharedOptional<JobsClient<Self::Block, Self::Backend, Self::Client>> {
        &self.jobs_client
    }

    fn pallet_tx(&self) -> Arc<dyn PalletSubmitter> {
        self.pallet_tx.clone()
    }

    fn logger(&self) -> DebugLogger {
        self.logger.clone()
    }

    fn client(&self) -> Self::Client {
        self.client.clone()
    }

    fn get_work_manager_config(&self) -> WorkManagerConfig {
        WorkManagerConfig {
            interval: Some(constants::signing_worker::JOB_POLL_INTERVAL),
            max_active_tasks: constants::signing_worker::MAX_RUNNING_TASKS,
            max_pending_tasks: constants::signing_worker::MAX_ENQUEUED_TASKS,
        }
    }
}

#[async_trait]
impl<
        B: Block,
        BE: Backend<B> + 'static,
        C: ClientWithApi<B, BE>,
        N: Network,
        KBE: KeystoreBackend,
    > FullProtocolConfig for DfnsKeyRefreshProtocol<B, BE, C, N, KBE>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    type AsyncProtocolParameters = DfnsCGGMP21KeyRefreshExtraParams;
    type Client = C;
    type Block = B;
    type Backend = BE;
    type Network = N;
    type AdditionalNodeParameters = ();
    type KeystoreBackend = KBE;

    async fn new(
        client: Self::Client,
        pallet_tx: Arc<dyn PalletSubmitter>,
        network: Self::Network,
        logger: DebugLogger,
        account_id: AccountId,
        key_store: ECDSAKeyStore<Self::KeystoreBackend>,
    ) -> Result<Self, Error> {
        Ok(Self {
            pallet_tx,
            logger,
            client,
            network,
            account_id,
            key_store,
            jobs_client: Arc::new(Mutex::new(None)),
        })
    }

    async fn generate_protocol_from(
        &self,
        associated_block_id: <WorkManager as WorkManagerInterface>::Clock,
        associated_retry_id: <WorkManager as WorkManagerInterface>::RetryID,
        associated_session_id: <WorkManager as WorkManagerInterface>::SessionID,
        associated_task_id: <WorkManager as WorkManagerInterface>::TaskID,
        protocol_message_rx: UnboundedReceiver<GadgetProtocolMessage>,
        additional_params: Self::AsyncProtocolParameters,
    ) -> Result<BuiltExecutableJobWrapper, JobError> {
        protocols::key_refresh::generate_protocol_from(
            self,
            associated_block_id,
            associated_retry_id,
            associated_session_id,
            associated_task_id,
            protocol_message_rx,
            additional_params,
        )
        .await
    }

    fn network(&self) -> &Self::Network {
        &self.network
    }

    async fn create_next_job(
        &self,
        job: JobInitMetadata<Self::Block>,
        _work_manager: &ProtocolWorkManager<WorkManager>,
    ) -> Result<Self::AsyncProtocolParameters, Error> {
        protocols::key_refresh::create_next_job(self, job).await
    }

    fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    fn name(&self) -> String {
        "DFNS-refresh-Protocol".to_string()
    }

    fn role_filter(&self, role: RoleType) -> bool {
        matches!(
            role,
            RoleType::Tss(ThresholdSignatureRoleType::DfnsCGGMP21Secp256k1)
        )
    }

    fn phase_filter(&self, job: GadgetJobType) -> bool {
        matches!(job, JobType::DKGTSSPhaseThree(_))
    }

    fn jobs_client(&self) -> &SharedOptional<JobsClient<Self::Block, Self::Backend, Self::Client>> {
        &self.jobs_client
    }

    fn pallet_tx(&self) -> Arc<dyn PalletSubmitter> {
        self.pallet_tx.clone()
    }

    fn logger(&self) -> DebugLogger {
        self.logger.clone()
    }

    fn client(&self) -> Self::Client {
        self.client.clone()
    }

    fn get_work_manager_config(&self) -> WorkManagerConfig {
        WorkManagerConfig {
            interval: None, // Manual polling
            max_active_tasks: constants::keygen_worker::MAX_RUNNING_TASKS,
            max_pending_tasks: constants::keygen_worker::MAX_ENQUEUED_TASKS,
        }
    }
}

#[async_trait]
impl<
        B: Block,
        BE: Backend<B> + 'static,
        C: ClientWithApi<B, BE>,
        N: Network,
        KBE: KeystoreBackend,
    > FullProtocolConfig for DfnsKeyRotateProtocol<B, BE, C, N, KBE>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    type AsyncProtocolParameters = DfnsCGGMP21KeyRotateExtraParams;
    type Client = C;
    type Block = B;
    type Backend = BE;
    type Network = N;
    type AdditionalNodeParameters = ();
    type KeystoreBackend = KBE;

    async fn new(
        client: Self::Client,
        pallet_tx: Arc<dyn PalletSubmitter>,
        network: Self::Network,
        logger: DebugLogger,
        account_id: AccountId,
        key_store: ECDSAKeyStore<Self::KeystoreBackend>,
    ) -> Result<Self, Error> {
        Ok(Self {
            pallet_tx,
            logger,
            client,
            network,
            account_id,
            key_store,
            jobs_client: Arc::new(Mutex::new(None)),
        })
    }

    async fn generate_protocol_from(
        &self,
        associated_block_id: <WorkManager as WorkManagerInterface>::Clock,
        associated_retry_id: <WorkManager as WorkManagerInterface>::RetryID,
        associated_session_id: <WorkManager as WorkManagerInterface>::SessionID,
        associated_task_id: <WorkManager as WorkManagerInterface>::TaskID,
        protocol_message_rx: UnboundedReceiver<GadgetProtocolMessage>,
        additional_params: Self::AsyncProtocolParameters,
    ) -> Result<BuiltExecutableJobWrapper, JobError> {
        protocols::key_rotate::generate_protocol_from(
            self,
            associated_block_id,
            associated_retry_id,
            associated_session_id,
            associated_task_id,
            protocol_message_rx,
            additional_params,
        )
        .await
    }

    fn network(&self) -> &Self::Network {
        &self.network
    }

    async fn create_next_job(
        &self,
        job: JobInitMetadata<Self::Block>,
        _work_manager: &ProtocolWorkManager<WorkManager>,
    ) -> Result<Self::AsyncProtocolParameters, Error> {
        protocols::key_rotate::create_next_job(self, job).await
    }

    fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    fn name(&self) -> String {
        "DFNS-rotation-Protocol".to_string()
    }

    fn role_filter(&self, role: RoleType) -> bool {
        matches!(
            role,
            RoleType::Tss(ThresholdSignatureRoleType::DfnsCGGMP21Secp256k1)
        )
    }

    fn phase_filter(&self, job: GadgetJobType) -> bool {
        matches!(job, JobType::DKGTSSPhaseFour(_))
    }

    fn jobs_client(&self) -> &SharedOptional<JobsClient<Self::Block, Self::Backend, Self::Client>> {
        &self.jobs_client
    }

    fn pallet_tx(&self) -> Arc<dyn PalletSubmitter> {
        self.pallet_tx.clone()
    }

    fn logger(&self) -> DebugLogger {
        self.logger.clone()
    }

    fn client(&self) -> Self::Client {
        self.client.clone()
    }

    fn get_work_manager_config(&self) -> WorkManagerConfig {
        WorkManagerConfig {
            interval: None, // Manual polling
            max_active_tasks: constants::keygen_worker::MAX_RUNNING_TASKS,
            max_pending_tasks: constants::keygen_worker::MAX_ENQUEUED_TASKS,
        }
    }
}

generate_setup_and_run_command!(
    DfnsKeygenProtocol,
    DfnsSigningProtocol,
    DfnsKeyRefreshProtocol,
    DfnsKeyRotateProtocol
);
