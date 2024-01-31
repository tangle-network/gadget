use crate::protocol::state_machine::{Group, GroupBlsful};
use async_trait::async_trait;
use blsful::{MultiPublicKey, MultiSignature, SignatureSchemes};
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
use gennaro_dkg::{Participant, SecretParticipantImpl};
use itertools::Itertools;
use round_based::Msg;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tangle_primitives::jobs::{
    DKGTSSSignatureResult, DigitalSignatureType, JobId, JobResult, JobType,
};
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
impl<
        B: Block,
        BE: Backend<B> + 'static,
        C: ClientWithApi<B, BE>,
        N: Network,
        KBE: KeystoreBackend,
    > GadgetProtocol<B, BE, C> for BlsSigningProtocol<B, BE, C, N, KBE>
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
    role_type: RoleType,
    job_id: JobId,
    key_bundle: Participant<SecretParticipantImpl<Group>, Group>,
    user_id_to_account_id_mapping: Arc<HashMap<UserID, AccountId>>,
    input_data_to_sign: Vec<u8>,
}

#[async_trait]
impl<
        B: Block,
        BE: Backend<B> + 'static,
        C: ClientWithApi<B, BE>,
        N: Network,
        KBE: KeystoreBackend,
    > AsyncProtocol for BlsSigningProtocol<B, BE, C, N, KBE>
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
        let threshold = additional_params.t;
        let network = self.network.clone();
        let result = Arc::new(tokio::sync::Mutex::new(None));
        let result_clone = result.clone();
        let client = self.jobs_client.clone();
        let role_type = additional_params.role_type;
        let job_id = additional_params.job_id;

        Ok(JobBuilder::new()
            .protocol(async move {
                let (_tx0, _rx0, tx1, mut rx1) =
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
                let sk = participant.get_secret_share().ok_or_else(|| JobError {
                    reason: "Failed to get secret share".to_string(),
                })?;

                let share_opt = blsful::SecretKey::<GroupBlsful>::from_be_bytes(&sk.to_be_bytes());
                if share_opt.is_none().into() {
                    return Err(JobError {
                        reason: "Failed to create secret key".to_string(),
                    });
                }

                let share = share_opt.unwrap();

                let sig_share = share
                    .sign(
                        SignatureSchemes::Basic,
                        &additional_params.input_data_to_sign,
                    )
                    .map_err(|e| JobError {
                        reason: format!("Failed to sign: {e}"),
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

                let mut received_pk_shares = BTreeMap::new();
                let mut received_sig_shares = BTreeMap::new();
                received_pk_shares.insert(additional_params.i, pk_share);

                // Step 3: Receive shares until there are t+1 total
                while received_pk_shares.len() != (threshold + 1) as usize {
                    let msg = rx1.recv().await.ok_or_else(|| JobError {
                        reason: "Failed to receive message".to_string(),
                    })?;

                    let (sender, (sig, pk)) = (msg.sender, msg.body);
                    received_pk_shares.insert(sender, pk);
                    received_sig_shares.insert(sender, sig);
                }

                // Step 4: Verify the combined signatures and public keys
                let sig_shares = received_sig_shares
                    .into_iter()
                    .sorted_by_key(|r| r.0)
                    .map(|r| r.1)
                    .collect::<Vec<_>>();

                let pk_shares = received_pk_shares
                    .into_iter()
                    .sorted_by_key(|r| r.0)
                    .map(|r| r.1)
                    .collect::<Vec<_>>();

                let combined_signature =
                    MultiSignature::<GroupBlsful>::from_signatures(&sig_shares).unwrap();
                let combined_pk = MultiPublicKey::<GroupBlsful>::from_public_keys(&pk_shares);
                combined_signature
                    .verify(combined_pk, &additional_params.input_data_to_sign)
                    .map_err(|e| JobError {
                        reason: format!("Failed to verify signature: {e}"),
                    })?;

                let job_result = JobResult::DKGPhaseTwo(DKGTSSSignatureResult {
                    signature_type: DigitalSignatureType::Bls381,
                    data: additional_params.input_data_to_sign.clone(),
                    signature: combined_signature.as_raw_value().to_uncompressed().to_vec(),
                    signing_key: share.to_be_bytes().to_vec(), // Use our secret share as the signing key
                });

                *result.lock().await = Some(job_result);
                Ok(())
            })
            .post(async move {
                if let Some(result) = result_clone.lock().await.take() {
                    client
                        .submit_job_result(role_type, job_id, result)
                        .await
                        .map_err(|e| JobError {
                            reason: format!("Failed to submit job result: {e}"),
                        })?;
                }

                Ok(())
            })
            .build())
    }
}
