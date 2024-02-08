use async_trait::async_trait;
use gadget_common::client::*;
use gadget_common::config::*;
use gadget_common::full_protocol::FullProtocolConfig;
use gadget_common::gadget::message::GadgetProtocolMessage;
use gadget_common::gadget::work_manager::WorkManager;
use gadget_common::gadget::JobInitMetadata;
use gadget_common::{BuiltExecutableJobWrapper, Error, JobBuilder, JobError, WorkManagerInterface};
use parking_lot::Mutex;
use protocol_macros::protocol;
use std::sync::Arc;
use tangle_primitives::roles::RoleType;
use tokio::sync::mpsc::UnboundedReceiver;

#[protocol]
pub struct StubConfig<B: Block, BE: Backend<B> + 'static, C: ClientWithApi<B, BE>, N: Network>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    pallet_tx: Arc<dyn PalletSubmitter>,
    logger: DebugLogger,
    client: C,
    network: N,
    account_id: AccountId,
    jobs_client: Arc<Mutex<Option<JobsClient<B, BE, C>>>>,
}

#[async_trait]
impl<B: Block, BE: Backend<B> + 'static, C: ClientWithApi<B, BE>, N: Network> FullProtocolConfig
    for StubConfig<B, BE, C, N>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    type AsyncProtocolParameters = ();
    type Client = C;
    type Block = B;
    type Backend = BE;

    async fn generate_protocol_from(
        &self,
        _associated_block_id: <WorkManager as WorkManagerInterface>::Clock,
        _associated_retry_id: <WorkManager as WorkManagerInterface>::RetryID,
        _associated_session_id: <WorkManager as WorkManagerInterface>::SessionID,
        _associated_task_id: <WorkManager as WorkManagerInterface>::TaskID,
        _protocol_message_rx: UnboundedReceiver<GadgetProtocolMessage>,
        _additional_params: Self::AsyncProtocolParameters,
    ) -> Result<BuiltExecutableJobWrapper, JobError> {
        Ok(JobBuilder::new().protocol(async move { Ok(()) }).build())
    }

    async fn initialize_network_and_protocol(
        &self,
        jobs_client: JobsClient<Self::Block, Self::Backend, Self::Client>,
    ) -> Result<(), Error> {
        self.jobs_client.lock().replace(jobs_client);
        Ok(())
    }

    async fn next_message(&self) -> Option<<WorkManager as WorkManagerInterface>::ProtocolMessage> {
        self.network.next_message().await
    }

    async fn send_message(
        &self,
        message: <WorkManager as WorkManagerInterface>::ProtocolMessage,
    ) -> Result<(), Error> {
        self.network.send_message(message).await
    }

    async fn create_next_job(
        &self,
        _job: JobInitMetadata<Self::Block>,
    ) -> Result<Self::AsyncProtocolParameters, Error> {
        Ok(())
    }

    fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    fn name(&self) -> String {
        "STUB-PROTOCOL".to_string()
    }

    fn role_filter(&self, _role: RoleType) -> bool {
        false
    }

    fn phase_filter(&self, _job: GadgetJobType) -> bool {
        false
    }

    fn jobs_client(&self) -> JobsClient<Self::Block, Self::Backend, Self::Client> {
        self.jobs_client
            .lock()
            .clone()
            .expect("Should be initialized")
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

pub async fn run<B: Block, BE: Backend<B> + 'static, C: ClientWithApi<B, BE>, N: Network>(
    client: C,
    pallet_tx: Arc<dyn PalletSubmitter>,
    network: N,
    logger: DebugLogger,
    account_id: AccountId,
) -> Result<(), Error>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    let config = StubConfig {
        pallet_tx,
        logger,
        client,
        network,
        account_id,
        jobs_client: Arc::new(Mutex::new(None)),
    };

    config.execute().await
}
