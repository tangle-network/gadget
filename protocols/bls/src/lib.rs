use crate::protocol::keygen::BlsKeygenAdditionalParams;
use async_trait::async_trait;
use gadget_common::full_protocol::SharedOptional;
use gadget_common::gadget::JobInitMetadata;
use gadget_common::prelude::*;
use gadget_common::{
    generate_setup_and_run_command, BuiltExecutableJobWrapper, Error, JobError,
    ProtocolWorkManager, WorkManagerInterface,
};
use protocol::signing::BlsSigningAdditionalParams;
use protocol_macros::protocol;
use std::sync::Arc;

pub mod protocol;

#[protocol]
pub struct BlsKeygenProtocol<
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
    > FullProtocolConfig for BlsKeygenProtocol<B, BE, C, N, KBE>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    type AsyncProtocolParameters = BlsKeygenAdditionalParams;
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
            jobs_client: Arc::new(parking_lot::Mutex::new(None)),
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
        protocol::keygen::generate_protocol_from(
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
        protocol::keygen::create_next_job(self, job).await
    }

    fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    fn name(&self) -> String {
        "BLS_KEYGEN_PROTOCOL".to_string()
    }

    fn role_filter(&self, role: RoleType) -> bool {
        matches!(
            role,
            RoleType::Tss(ThresholdSignatureRoleType::GennaroDKGBls381)
        )
    }

    fn phase_filter(&self, job: GadgetJobType) -> bool {
        matches!(job, GadgetJobType::DKGTSSPhaseOne(_))
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
}

#[protocol]
pub struct BlsSigningProtocol<
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
    _pd: std::marker::PhantomData<(B, BE)>,
}

#[async_trait]
impl<
        B: Block,
        BE: Backend<B> + 'static,
        C: ClientWithApi<B, BE>,
        N: Network,
        KBE: KeystoreBackend,
    > FullProtocolConfig for BlsSigningProtocol<B, BE, C, N, KBE>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    type AsyncProtocolParameters = BlsSigningAdditionalParams;
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
            jobs_client: Arc::new(parking_lot::Mutex::new(None)),
            _pd: std::marker::PhantomData,
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
        protocol::signing::generate_protocol_from(
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
        protocol::signing::create_next_job(self, job).await
    }

    fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    fn name(&self) -> String {
        "BLS_SIGNING_PROTOCOL".to_string()
    }

    fn role_filter(&self, role: RoleType) -> bool {
        matches!(
            role,
            RoleType::Tss(ThresholdSignatureRoleType::GennaroDKGBls381)
        )
    }

    fn phase_filter(&self, job: GadgetJobType) -> bool {
        matches!(job, GadgetJobType::DKGTSSPhaseTwo(_))
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
}

generate_setup_and_run_command!(BlsKeygenProtocol, BlsSigningProtocol);
test_utils::generate_signing_and_keygen_tss_tests!(
    2,
    3,
    ThresholdSignatureRoleType::GennaroDKGBls381
);
