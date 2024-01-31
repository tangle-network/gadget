use crate::protocol::state_machine::{Group, GroupBlsful};
use async_trait::async_trait;
use blsful::inner_types::elliptic_curve::PublicKey;
use blsful::{Signature, SignatureSchemes};
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
use gennaro_dkg::{Parameters, Participant, SecretParticipantImpl};
use round_based::Msg;
use std::collections::{BTreeMap, HashMap};
use std::num::NonZeroUsize;
use std::sync::Arc;
use tangle_primitives::jobs::{JobId, JobType};
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
        let JobType::DKGTSSPhaseTwo(p2_job) = job.job_type else {
            panic!("Should be valid type")
        };
        let input_data_to_sign = p2_job.submission;
        let previous_job_id = p2_job.phase_one_id;

        let user_id_to_account_id_mapping = Arc::new(
            participants
                .clone()
                .into_iter()
                .enumerate()
                .map(|r| (r.0 as UserID, r.1))
                .collect(),
        );

        if let Ok(Some(key_bundle)) = self
            .keystore
            .get::<Participant<SecretParticipantImpl<Group>, Group>>(&previous_job_id)
            .await
        {
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
                key_bundle,
                input_data_to_sign,
            };

            Ok(additional_params)
        } else {
            Err(Error::ClientError {
                err: "Key bundle not found".to_string(),
            })
        }
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
    key_bundle: Participant<SecretParticipantImpl<Group>, Group>,
    user_id_to_account_id_mapping: Arc<HashMap<UserID, AccountId>>,
    input_data_to_sign: Vec<u8>,
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
        associated_block_id: <WorkManager as WorkManagerInterface>::Clock,
        associated_retry_id: <WorkManager as WorkManagerInterface>::RetryID,
        associated_session_id: <WorkManager as WorkManagerInterface>::SessionID,
        associated_task_id: <WorkManager as WorkManagerInterface>::TaskID,
        protocol_message_rx: tokio::sync::mpsc::UnboundedReceiver<GadgetProtocolMessage>,
        additional_params: Self::AdditionalParams,
    ) -> Result<BuiltExecutableJobWrapper, JobError> {
        let threshold = NonZeroUsize::new(additional_params.t as usize).expect(" T should be > 0");
        let n = NonZeroUsize::new(additional_params.n as usize).expect("N should be > 0");
        let network = self.network.clone();
        let keystore = self.keystore.clone();

        Ok(JobBuilder::new()
            .protocol(async move {
                let (_tx0, _rx0, tx1, rx1) =
                    gadget_common::channels::create_job_manager_to_async_protocol_channel_split::<
                        _,
                        (),
                        Msg<(
                            blsful::Signature<GroupBlsful>,
                            blsful::PublicKey<GroupBlsful>,
                        )>,
                    >(
                        protocol_message_rx,
                        associated_block_id,
                        associated_retry_id,
                        associated_session_id,
                        associated_task_id,
                        additional_params.user_id_to_account_id_mapping.clone(),
                        network,
                    );

                // Step 1: Generate shares
                let participant = &additional_params.key_bundle;
                let sk = participant
                    .get_secret_share()
                    .ok_or_else(|| JobError::ClientError {
                        err: "Failed to get secret share".to_string(),
                    })?;

                let share_opt = blsful::SecretKey::<GroupBlsful>::from_be_bytes(&sk.to_be_bytes());
                if share_opt.is_none() {
                    return Err(JobError::ClientError {
                        err: "Failed to create secret key".to_string(),
                    });
                }

                let share = share_opt.unwrap();

                let sig_share = share
                    .sign(
                        SignatureSchemes::Basic,
                        &additional_params.input_data_to_sign,
                    )
                    .map_err(|e| JobError::ClientError {
                        err: format!("Failed to sign: {e}"),
                    })?;
                let pk_share = blsful::PublicKey::<GroupBlsful>::from(&share);

                // Step 2: Broadcast shares
                let msg = Msg {
                    sender: additional_params.i,
                    receiver: None,
                    body: (sig_share.clone(), pk_share.clone()),
                };

                tx1.send(msg).map_err(|e| JobError {
                    reason: format!("Failed to send message: {e}"),
                })?;

                let mut received_shares = BTreeMap::new();
                received_shares.insert(additional_params.i, (sig_share, pk_share));

                Ok(())
            })
            .build())
    }
}
