use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use gadget_common::client::{AccountId, ClientWithApi, JobsClient};
use gadget_common::config::{DebugLogger, JobsApi, ProvideRuntimeApi};
use gadget_common::gadget::message::GadgetProtocolMessage;
use gadget_common::gadget::work_manager::WorkManager;
use gadget_common::gadget::{GadgetProtocol, JobInitMetadata};
use gadget_common::protocol::AsyncProtocol;
use gadget_common::gadget::message::UserID;
use gadget_common::{
    Backend, Block, BuiltExecutableJobWrapper, Error, JobBuilder, JobError, ProtocolWorkManager, WorkManagerInterface
};
use sc_client_api::BlockImportNotification;
use tangle_primitives::jobs::{JobId, JobType};
use tangle_primitives::roles::RoleType;

pub struct ZcashFrostProtocol<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    pub jobs_client: JobsClient<B, BE, C>,
    pub account_id: AccountId,
    pub logger: DebugLogger,
}

pub type Curve = u8;

pub struct ZcashFrostKeygenExtraParams {
    i: u16,
    t: u16,
    n: u16,
    job_id: JobId,
    role_type: RoleType,
    user_id_to_account_id_mapping: Arc<HashMap<UserID, AccountId>>,
}

#[async_trait]
impl<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>> GadgetProtocol<B, BE, C>
    for ZcashFrostProtocol<B, BE, C>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    async fn create_next_job(
        &self,
        job: JobInitMetadata,
    ) -> Result<<Self as AsyncProtocol>::AdditionalParams, Error> {
        let now = job.now;
        self.logger.info(format!("At finality notification {now}"));

        let job_id = job.job_id;
        let role_type = job.job_type.get_role_type();
        // ZcashFrostSr25519 | ZcashFrostP256 | ZcashFrostSecp256k1 | ZcashFrostRistretto255 | ZcashFrostEd25519
        

        // We can safely make this assumption because we are only creating jobs for phase one
        let JobType::DKGTSSPhaseOne(p1_job) = job.job_type else {
            panic!("Should be valid type")
        };

        let participants = p1_job.participants;
        let threshold = p1_job.threshold;

        let user_id_to_account_id_mapping = Arc::new(
            participants
                .clone()
                .into_iter()
                .enumerate()
                .map(|r| (r.0 as UserID, r.1))
                .collect(),
        );

        let params = ZcashFrostKeygenExtraParams {
            i: participants
                .iter()
                .position(|p| p == &self.account_id)
                .expect("Should exist") as u16,
            t: threshold as u16,
            n: participants.len() as u16,
            role_type,
            job_id,
            user_id_to_account_id_mapping,
        };

        Ok(params)
    }

    async fn process_block_import_notification(
        &self,
        _notification: BlockImportNotification<B>,
        _job_manager: &ProtocolWorkManager<WorkManager>,
    ) -> Result<(), Error> {
        Ok(())
    }

    async fn process_error(&self, _error: Error, _job_manager: &ProtocolWorkManager<WorkManager>) {}

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
impl<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>> AsyncProtocol for ZcashFrostProtocol<B, BE, C>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    type AdditionalParams = ZcashFrostKeygenExtraParams;

    async fn generate_protocol_from(
        &self,
        _associated_block_id: <WorkManager as WorkManagerInterface>::Clock,
        _associated_retry_id: <WorkManager as WorkManagerInterface>::RetryID,
        _associated_session_id: <WorkManager as WorkManagerInterface>::SessionID,
        _associated_task_id: <WorkManager as WorkManagerInterface>::TaskID,
        _protocol_message_rx: tokio::sync::mpsc::UnboundedReceiver<GadgetProtocolMessage>,
        _additional_params: Self::AdditionalParams,
    ) -> Result<BuiltExecutableJobWrapper, JobError> {
        Ok(JobBuilder::new().protocol(async move { Ok(()) }).build())
    }
}
