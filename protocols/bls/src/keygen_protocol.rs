use std::collections::{BTreeMap, HashMap};
use std::num::NonZeroUsize;
use std::sync::Arc;
use async_trait::async_trait;
use gennaro_dkg::{Parameters, SecretParticipant};
use tangle_primitives::jobs::JobId;
use gadget_common::client::{AccountId, ClientWithApi, JobsClient};
use gadget_common::config::{DebugLogger, GadgetProtocol, JobsApi, Network, ProvideRuntimeApi};
use gadget_common::gadget::message::{GadgetProtocolMessage, UserID};
use gadget_common::gadget::work_manager::WorkManager;
use gadget_common::gadget::JobInitMetadata;
use gadget_common::protocol::AsyncProtocol;
use gadget_common::{
    Backend, Block, BlockImportNotification, BuiltExecutableJobWrapper, Error, JobBuilder,
    JobError, ProtocolWorkManager, WorkManagerInterface,
};
use tangle_primitives::roles::{RoleType, ThresholdSignatureRoleType};
use vsss_rs::elliptic_curve::weierstrass::add;
use gadget_common::keystore::{GenericKeyStore, KeystoreBackend};

pub struct BlsKeygenProtocol<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>, N: Network, KBE: KeystoreBackend>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    pub jobs_client: JobsClient<B, BE, C>,
    pub account_id: AccountId,
    pub logger: DebugLogger,
    pub network: N,
    pub keystore: GenericKeyStore<KBE, gadget_common::sp_core::ecdsa::Pair>
}

pub type Group = bls12_381_plus::G1Projective;

#[async_trait]
impl<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>, N: Network, KBE: KeystoreBackend> GadgetProtocol<B, BE, C>
    for BlsKeygenProtocol<B, BE, C, N, KBE>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    async fn create_next_job(
        &self,
        job: JobInitMetadata,
    ) -> Result<<Self as AsyncProtocol>::AdditionalParams, Error> {
        let job_id = job.job_id;
        let p1_job = job.job_type;
        let threshold = p1_job.get_threshold().expect("Should exist") as u16;
        let role_type = p1_job.get_role_type();
        let participants = p1_job.get_participants().expect("Should exist");
        let user_id_to_account_id_mapping = Arc::new(
            participants
                .clone()
                .into_iter()
                .enumerate()
                .map(|r| (r.0 + 1 as UserID, r.1))
                .collect(),
        );

        let additional_params = BlsKeygenAdditionalParams {
            i: (participants
                .iter()
                .position(|p| p == &self.account_id)
                .expect("Should exist") + 1) as u16,
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
        true
    }

    fn client(&self) -> &JobsClient<B, BE, C> {
        &self.jobs_client
    }

    fn logger(&self) -> &DebugLogger {
        &self.logger
    }
}

#[derive(Clone)]
pub struct BlsKeygenAdditionalParams {
    i: u16,
    t: u16,
    n: u16,
    role_type: RoleType,
    job_id: JobId,
    user_id_to_account_id_mapping: Arc<HashMap<UserID, AccountId>>,
}

#[async_trait]
impl<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>, N: Network, KBE: KeystoreBackend> AsyncProtocol for BlsKeygenProtocol<B, BE, C, N, KBE>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    type AdditionalParams = BlsKeygenAdditionalParams;

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
        let i = NonZeroUsize::new(additional_params.i as usize).expect("I should be > 0");
        Ok(JobBuilder::new().protocol(async move {
            let params = Parameters::<Group>::new(threshold, n);
            let mut me = SecretParticipant::<Group>::new(i, params).unwrap();
            tokio::task::spawn_blocking(move || {
                // round 1: Everyone generated broadcast payload + p2p payload, then, sends to everyone
                let count_required = additional_params.n - 1;
                let (broadcast, p2p) = me.round1().map_err(|e| JobError { reason: e.to_string() })?;
                let mut broadcast_data_r1 = BTreeMap::new();
                let mut p2p_data_r1 = BTreeMap::new();
                broadcast_data_r1.insert(additional_params.i, broadcast);
                p2p_data_r1.insert(additional_params.i, p2p);

                for _ in 0..count_required {

                }

            }).await.unwrap();

        }).build())
    }
}
