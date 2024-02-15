use crate::protocol::state_machine::Group;
use gadget_common::client::{AccountId, ClientWithApi, JobsApiForGadget};
use gadget_common::config::{Network, ProvideRuntimeApi};
use gadget_common::full_protocol::FullProtocolConfig;
use gadget_common::gadget::message::{GadgetProtocolMessage, UserID};
use gadget_common::gadget::work_manager::WorkManager;
use gadget_common::gadget::JobInitMetadata;
use gadget_common::keystore::KeystoreBackend;
use gadget_common::sp_core::keccak_256;
use gadget_common::{
    Backend, Block, BuiltExecutableJobWrapper, Error, JobBuilder, JobError, WorkManagerInterface,
};
use gennaro_dkg::{Participant, SecretParticipantImpl};
use itertools::Itertools;
use round_based::Msg;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tangle_primitives::jobs::{
    DKGTSSSignatureResult, DigitalSignatureScheme, JobId, JobResult, JobType,
};
use tangle_primitives::roles::RoleType;

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

pub async fn create_next_job<
    B: Block,
    BE: Backend<B> + 'static,
    C: ClientWithApi<B, BE>,
    N: Network,
    KBE: KeystoreBackend,
>(
    config: &crate::BlsSigningConfig<B, BE, C, N, KBE>,
    job: JobInitMetadata<B>,
) -> Result<BlsSigningAdditionalParams, Error>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
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

    let key = keccak_256(&previous_job_id.to_be_bytes());

    if let Ok(Some(key_bundle)) = config
        .key_store
        .get::<Participant<SecretParticipantImpl<Group>, Group>>(&key)
        .await
    {
        let additional_params = BlsSigningAdditionalParams {
            i: participants
                .iter()
                .position(|p| p == &config.account_id)
                .expect("Should exist") as u16,
            t: threshold,
            role_type,
            job_id,
            user_id_to_account_id_mapping,
            key_bundle,
            input_data_to_sign: input_data_to_sign.to_vec(),
        };

        Ok(additional_params)
    } else {
        Err(Error::ClientError {
            err: "Key bundle not found".to_string(),
        })
    }
}

pub async fn generate_protocol_from<
    B: Block,
    BE: Backend<B> + 'static,
    C: ClientWithApi<B, BE>,
    N: Network,
    KBE: KeystoreBackend,
>(
    config: &crate::BlsSigningConfig<B, BE, C, N, KBE>,
    associated_block_id: <WorkManager as WorkManagerInterface>::Clock,
    associated_retry_id: <WorkManager as WorkManagerInterface>::RetryID,
    associated_session_id: <WorkManager as WorkManagerInterface>::SessionID,
    associated_task_id: <WorkManager as WorkManagerInterface>::TaskID,
    protocol_message_rx: tokio::sync::mpsc::UnboundedReceiver<GadgetProtocolMessage>,
    additional_params: BlsSigningAdditionalParams,
) -> Result<BuiltExecutableJobWrapper, JobError>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    let threshold = additional_params.t;
    let network = config.network.clone();
    let result = Arc::new(tokio::sync::Mutex::new(None));
    let result_clone = result.clone();
    let client = config.jobs_client();
    let role_type = additional_params.role_type;
    let job_id = additional_params.job_id;
    let logger = config.logger.clone();

    Ok(JobBuilder::new()
        .protocol(async move {
            logger.info("Starting BlsSigningProtocol");
            let (_tx0, _rx0, tx1, mut rx1) =
                gadget_common::channels::create_job_manager_to_async_protocol_channel_split::<
                    _,
                    (),
                    Msg<(Vec<u8>, Vec<u8>)>,
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

            let share =
                snowbridge_milagro_bls::SecretKey::from_bytes(&sk.to_be_bytes()).map_err(|e| {
                    JobError {
                        reason: format!("Failed to create secret key: {e:?}"),
                    }
                })?;

            let sign_input = keccak_256(&additional_params.input_data_to_sign);
            let sig_share = snowbridge_milagro_bls::Signature::new(&sign_input, &share);
            let pk_share = snowbridge_milagro_bls::PublicKey::from_secret_key(&share);

            // Step 2: Broadcast shares
            let msg = Msg {
                sender: additional_params.i,
                receiver: None,
                body: (sig_share.as_bytes().to_vec(), pk_share.as_bytes().to_vec()),
            };

            tx1.send(msg).map_err(|e| JobError {
                reason: format!("Failed to send message: {e}"),
            })?;

            let mut received_pk_shares = BTreeMap::new();
            let mut received_sig_shares = BTreeMap::new();
            received_pk_shares.insert(additional_params.i, pk_share);
            received_sig_shares.insert(additional_params.i, sig_share);

            // Step 3: Receive shares until there are t+1 total
            while received_pk_shares.len() != (threshold + 1) as usize {
                let msg = rx1.recv().await.ok_or_else(|| JobError {
                    reason: "Failed to receive message".to_string(),
                })?;

                let (sender, (sig, pk)) = (msg.sender, msg.body);
                let pk =
                    snowbridge_milagro_bls::PublicKey::from_bytes(&pk).map_err(|e| JobError {
                        reason: format!("Failed to create public key: {e:?}"),
                    })?;
                let sig =
                    snowbridge_milagro_bls::Signature::from_bytes(&sig).map_err(|e| JobError {
                        reason: format!("Failed to create signature: {e:?}"),
                    })?;
                received_pk_shares.insert(sender, pk);
                received_sig_shares.insert(sender, sig);
            }

            logger.info("BlsSigningProtocol finished public broadcast stage");

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

            let combined_signature = snowbridge_milagro_bls::AggregateSignature::aggregate(
                &sig_shares.iter().collect::<Vec<_>>(),
            );

            let pk_agg = snowbridge_milagro_bls::AggregatePublicKey::aggregate(
                &pk_shares.iter().collect::<Vec<_>>(),
            )
            .map_err(|e| JobError {
                reason: format!("Failed to aggregate public keys: {e:?}"),
            })?;

            let input = &mut [0u8; 97];
            pk_agg.point.to_bytes(input, false);

            let as_pk = snowbridge_milagro_bls::PublicKey::from_uncompressed_bytes(&input[1..])
                .map_err(|e| JobError {
                    reason: format!("Failed to create public key: {e:?}"),
                })?;

            let as_sig =
                snowbridge_milagro_bls::Signature::from_bytes(&combined_signature.as_bytes())
                    .map_err(|e| JobError {
                        reason: format!("Failed to create signature: {e:?}"),
                    })?;

            if !as_sig.verify(&sign_input, &as_pk) {
                return Err(JobError {
                    reason: "Failed to verify signature locally".to_string(),
                });
            }

            let signing_key = as_pk.as_uncompressed_bytes().to_vec();
            let signature = as_sig.as_bytes().to_vec();

            logger.info("BlsSigningProtocol finished verification stage");
            let job_result = JobResult::DKGPhaseTwo(DKGTSSSignatureResult {
                signature_scheme: DigitalSignatureScheme::Bls381,
                data: additional_params
                    .input_data_to_sign
                    .clone()
                    .try_into()
                    .unwrap(),
                signature: signature.try_into().unwrap(),
                verifying_key: signing_key.try_into().unwrap(),
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
