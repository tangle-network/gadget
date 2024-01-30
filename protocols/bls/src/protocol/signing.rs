use async_trait::async_trait;
use gadget_common::client::{AccountId, ClientWithApi, JobsClient};
use gadget_common::config::{DebugLogger, GadgetProtocol, JobsApi, Network, ProvideRuntimeApi};
use gadget_common::gadget::message::{GadgetProtocolMessage, UserID};
use gadget_common::gadget::work_manager::WorkManager;
use gadget_common::gadget::JobInitMetadata;
use gadget_common::keystore::{GenericKeyStore, KeystoreBackend};
use gadget_common::protocol::AsyncProtocol;
use gadget_common::{
    Backend, Block, BlockImportNotification, BuiltExecutableJobWrapper, Error, JobBuilder,
    JobError, ProtocolWorkManager, WorkManagerInterface,
};
use gennaro_dkg::Parameters;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;
use tangle_primitives::jobs::JobId;
use tangle_primitives::roles::{RoleType, ThresholdSignatureRoleType};

pub struct BlsSigningProtocol<
    B: Block,
    BE: Backend<B>,
    C: ClientWithApi<B, BE>,
    N: Network,
    KBE: KeystoreBackend,
> where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    pub jobs_client: JobsClient<B, BE, C>,
    pub account_id: AccountId,
    pub logger: DebugLogger,
    pub network: N,
    pub keystore: GenericKeyStore<KBE, gadget_common::sp_core::ecdsa::Pair>,
}

#[async_trait]
impl<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>, N: Network, KBE: KeystoreBackend>
    GadgetProtocol<B, BE, C> for BlsSigningProtocol<B, BE, C, N, KBE>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    async fn create_next_job(
        &self,
        job: JobInitMetadata,
    ) -> Result<<Self as AsyncProtocol>::AdditionalParams, Error> {
        let job_id = job.job_id;
        let p1_job = job.phase1_job.expect("Should exist");
        let threshold = p1_job.clone().get_threshold().expect("Should exist") as u16;
        let role_type = p1_job.get_role_type();
        let participants = p1_job.get_participants().expect("Should exist");
        let user_id_to_account_id_mapping = Arc::new(
            participants
                .clone()
                .into_iter()
                .enumerate()
                .map(|r| (r.0 as UserID, r.1))
                .collect(),
        );

        let additional_params = BlsSigningAdditionalParams {
            i: participants
                .iter()
                .position(|p| p == &self.account_id)
                .expect("Should exist") as u16,
            t: threshold,
            n: participants.len() as u16,
            role_type,
            job_id,
            user_id_to_account_id_mapping,
        };

        Ok(additional_params)
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
        RoleType::Tss(ThresholdSignatureRoleType::GennaroDKGBls381)
    }

    fn is_phase_one(&self) -> bool {
        false
    }

    fn client(&self) -> &JobsClient<B, BE, C> {
        &self.jobs_client
    }

    fn logger(&self) -> &DebugLogger {
        &self.logger
    }
}

#[derive(Clone)]
pub struct BlsSigningAdditionalParams {
    i: u16,
    t: u16,
    n: u16,
    role_type: RoleType,
    job_id: JobId,
    user_id_to_account_id_mapping: Arc<HashMap<UserID, AccountId>>,
}

#[async_trait]
impl<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>, N: Network, KBE: KeystoreBackend>
    AsyncProtocol for BlsSigningProtocol<B, BE, C, N, KBE>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    type AdditionalParams = BlsSigningAdditionalParams;

    async fn generate_protocol_from(
        &self,
        _associated_block_id: <WorkManager as WorkManagerInterface>::Clock,
        _associated_retry_id: <WorkManager as WorkManagerInterface>::RetryID,
        _associated_session_id: <WorkManager as WorkManagerInterface>::SessionID,
        _associated_task_id: <WorkManager as WorkManagerInterface>::TaskID,
        _protocol_message_rx: tokio::sync::mpsc::UnboundedReceiver<GadgetProtocolMessage>,
        additional_params: Self::AdditionalParams,
    ) -> Result<BuiltExecutableJobWrapper, JobError> {
        let threshold = NonZeroUsize::new(additional_params.t as usize).expect(" T should be > 0");
        let n = NonZeroUsize::new(additional_params.n as usize).expect("N should be > 0");
        Ok(JobBuilder::new()
            .protocol(async move {
                let _params = Parameters::<bls12_381_plus::G1Projective>::new(threshold, n);
                Ok(())
            })
            .build())
    }
}
