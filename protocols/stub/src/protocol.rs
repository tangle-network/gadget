use async_trait::async_trait;
use gadget_common::client::{AccountId, ClientWithApi, JobsClient};
use gadget_common::config::{DebugLogger, JobsApi, ProvideRuntimeApi, WebbGadgetProtocol};
use gadget_common::gadget::message::GadgetProtocolMessage;
use gadget_common::gadget::work_manager::WebbWorkManager;
use gadget_common::gadget::JobInitMetadata;
use gadget_common::protocol::AsyncProtocol;
use gadget_common::{
    Backend, Block, BlockImportNotification, BuiltExecutableJobWrapper, Error, JobBuilder,
    JobError, ProtocolWorkManager, WorkManagerInterface,
};
use tangle_primitives::roles::RoleType;

pub struct StubProtocolImpl<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    pub jobs_client: JobsClient<B, BE, C>,
    pub account_id: AccountId,
    pub logger: DebugLogger,
}

#[async_trait]
impl<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>> WebbGadgetProtocol<B, BE, C>
    for StubProtocolImpl<B, BE, C>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    async fn create_next_job(
        &self,
        _job: JobInitMetadata,
    ) -> Result<<Self as AsyncProtocol>::AdditionalParams, Error> {
        Ok(())
    }

    async fn process_block_import_notification(
        &self,
        _notification: BlockImportNotification<B>,
        _job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) -> Result<(), Error> {
        Ok(())
    }

    async fn process_error(
        &self,
        _error: Error,
        _job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) {
    }

    fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    fn role_type(&self) -> RoleType {
        RoleType::LightClientRelaying
    }

    fn is_phase_one(&self) -> bool {
        true
    }

    fn client(&self) -> &JobsClient<B, BE, C> {
        &self.jobs_client
    }

    fn logger(&self) -> &DebugLogger {
        &self.logger
    }
}

#[async_trait]
impl<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>> AsyncProtocol for StubProtocolImpl<B, BE, C>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    type AdditionalParams = ();

    async fn generate_protocol_from(
        &self,
        _associated_block_id: <WebbWorkManager as WorkManagerInterface>::Clock,
        _associated_retry_id: <WebbWorkManager as WorkManagerInterface>::RetryID,
        _associated_session_id: <WebbWorkManager as WorkManagerInterface>::SessionID,
        _associated_task_id: <WebbWorkManager as WorkManagerInterface>::TaskID,
        _protocol_message_rx: tokio::sync::mpsc::UnboundedReceiver<GadgetProtocolMessage>,
        _additional_params: Self::AdditionalParams,
    ) -> Result<BuiltExecutableJobWrapper, JobError> {
        Ok(JobBuilder::new().protocol(async move { Ok(()) }).build())
    }
}
