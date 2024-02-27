use dfns_cggmp21::supported_curves::Secp256k1;
use dfns_cggmp21::KeyShare;
use gadget_common::client::{AccountId, ClientWithApi, JobsApiForGadget};
use gadget_common::gadget::message::{GadgetProtocolMessage, UserID};
use gadget_common::gadget::work_manager::WorkManager;
use gadget_common::gadget::JobInitMetadata;
use gadget_common::keystore::KeystoreBackend;
use gadget_common::prelude::{FullProtocolConfig, Network};
use gadget_common::utils::CloneableUnboundedReceiver;
use gadget_common::Block;
use gadget_core::job::{BuiltExecutableJobWrapper, JobBuilder, JobError};
use gadget_core::job_manager::{ProtocolWorkManager, WorkManagerInterface};
use rand::SeedableRng;
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_core::keccak_256;
use std::collections::HashMap;
use std::sync::Arc;
use tangle_primitives::jobs::{
    DKGTSSSignatureResult, DigitalSignatureScheme, JobId, JobResult, JobType,
};
use tangle_primitives::roles::RoleType;
use tokio::sync::mpsc::UnboundedReceiver;

pub async fn create_next_job<
    B: Block,
    BE: Backend<B> + 'static,
    KBE: KeystoreBackend,
    C: ClientWithApi<B, BE>,
    N: Network,
>(
    config: &crate::DfnsSigningProtocol<B, BE, C, N, KBE>,
    job: JobInitMetadata<B>,
    _work_manager: &ProtocolWorkManager<WorkManager>,
) -> Result<DfnsCGGMP21SigningExtraParams, gadget_common::Error>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    let job_id = job.job_id;

    let JobType::DKGTSSPhaseTwo(p2_job) = job.job_type else {
        panic!("Should be valid type")
    };
    let input_data_to_sign = p2_job.submission.to_vec();
    let previous_job_id = p2_job.phase_one_id;

    let phase1_job = job.phase1_job.expect("Should exist for a phase 2 job");
    let participants = phase1_job.clone().get_participants().expect("Should exist");
    let t = phase1_job.get_threshold().expect("Should exist") as u16;

    let seed = keccak_256(&[&job_id.to_be_bytes()[..], &job.retry_id.to_be_bytes()[..]].concat());
    let mut rng = rand_chacha::ChaChaRng::from_seed(seed);

    let (i, signers, mapping) =
        super::util::choose_signers(&mut rng, &config.account_id, &participants, t)?;
    let key = config
        .key_store
        .get_job_result(previous_job_id)
        .await
        .map_err(|err| gadget_common::Error::ClientError {
            err: err.to_string(),
        })?
        .ok_or_else(|| gadget_common::Error::ClientError {
            err: format!("No key found for job ID: {job_id:?}"),
        })?;

    let user_id_to_account_id_mapping = Arc::new(mapping);

    let params = DfnsCGGMP21SigningExtraParams {
        i,
        t,
        signers,
        job_id,
        role_type: job.role_type,
        key,
        input_data_to_sign,
        user_id_to_account_id_mapping,
    };
    Ok(params)
}

#[derive(Clone)]
pub struct DfnsCGGMP21SigningExtraParams {
    i: u16,
    t: u16,
    signers: Vec<u16>,
    job_id: JobId,
    role_type: RoleType,
    key: KeyShare<Secp256k1>,
    input_data_to_sign: Vec<u8>,
    user_id_to_account_id_mapping: Arc<HashMap<UserID, AccountId>>,
}

pub async fn generate_protocol_from<
    B: Block,
    BE: Backend<B> + 'static,
    KBE: KeystoreBackend,
    C: ClientWithApi<B, BE>,
    N: Network,
>(
    config: &crate::DfnsSigningProtocol<B, BE, C, N, KBE>,
    associated_block_id: <WorkManager as WorkManagerInterface>::Clock,
    associated_retry_id: <WorkManager as WorkManagerInterface>::RetryID,
    associated_session_id: <WorkManager as WorkManagerInterface>::SessionID,
    associated_task_id: <WorkManager as WorkManagerInterface>::TaskID,
    protocol_message_channel: UnboundedReceiver<GadgetProtocolMessage>,
    additional_params: DfnsCGGMP21SigningExtraParams,
) -> Result<BuiltExecutableJobWrapper, JobError>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    let debug_logger_post = config.logger.clone();
    let logger = debug_logger_post.clone();
    let protocol_output = Arc::new(tokio::sync::Mutex::new(None));
    let protocol_output_clone = protocol_output.clone();
    let client = config.get_jobs_client();
    let id = config.account_id;
    let network = config.clone();

    let (i, signers, t, key, input_data_to_sign, mapping) = (
        additional_params.i,
        additional_params.signers,
        additional_params.t,
        additional_params.key,
        additional_params.input_data_to_sign.clone(),
        additional_params.user_id_to_account_id_mapping.clone(),
    );

    let public_key_bytes = key.shared_public_key().to_bytes(true).to_vec();
    let input_data_to_sign2 = input_data_to_sign.clone();

    Ok(JobBuilder::new()
        .protocol(async move {
            let mut rng = rand::rngs::StdRng::from_entropy();
            let protocol_message_channel =
                CloneableUnboundedReceiver::from(protocol_message_channel);

            logger.info(format!(
                "Starting Signing Protocol with params: i={i}, t={t}"
            ));

            let job_id_bytes = additional_params.job_id.to_be_bytes();
            let mix = keccak_256(b"dnfs-cggmp21-signing");
            let eid_bytes = [&job_id_bytes[..], &mix[..]].concat();
            let eid = dfns_cggmp21::ExecutionId::new(&eid_bytes);
            let (
                signing_tx_to_outbound,
                signing_rx_async_proto,
                _broadcast_tx_to_outbound,
                _broadcast_rx_from_gadget,
            ) = super::util::create_job_manager_to_async_protocol_channel_split_futures::<_, (), _>(
                protocol_message_channel.clone(),
                associated_block_id,
                associated_retry_id,
                associated_session_id,
                associated_task_id,
                mapping.clone(),
                id,
                network.clone(),
            );

            let mut tracer = dfns_cggmp21::progress::PerfProfiler::new();
            let delivery = (signing_rx_async_proto, signing_tx_to_outbound);
            let party = dfns_cggmp21::round_based::MpcParty::connected(delivery);
            let data_hash = keccak_256(&input_data_to_sign);
            let data_to_sign = dfns_cggmp21::DataToSign::from_scalar(
                dfns_cggmp21::generic_ec::Scalar::from_be_bytes_mod_order(data_hash),
            );
            let signature = dfns_cggmp21::signing(eid, i, &signers, &key)
                .set_progress_tracer(&mut tracer)
                .sign(&mut rng, party, data_to_sign)
                .await
                .map_err(|err| JobError {
                    reason: format!("Signing protocol error: {err:?}"),
                })?;

            let perf_report = tracer.get_report().map_err(|err| JobError {
                reason: format!("Signing protocol error: {err:?}"),
            })?;
            logger.trace(format!("Signing protocol report: {perf_report}"));
            // Normalize the signature
            let signature = signature.normalize_s();
            logger.debug("Finished AsyncProtocol - Signing");
            *protocol_output.lock().await = Some(signature);
            Ok(())
        })
        .post(async move {
            // Submit the protocol output to the blockchain
            if let Some(signature) = protocol_output_clone.lock().await.take() {
                let mut signature_bytes = [0u8; 65];
                signature.write_to_slice(&mut signature_bytes[0..64]);
                // To figure out the recovery ID, we need to try all possible values of v
                // in our case, v can be 0 or 1
                let mut v = 0u8;
                loop {
                    let mut signature_bytes = signature_bytes;
                    let data_hash = keccak_256(&input_data_to_sign2);
                    signature_bytes[64] = v;
                    let res = sp_io::crypto::secp256k1_ecdsa_recover(&signature_bytes, &data_hash);
                    match res {
                        Ok(key) if key[..32] == public_key_bytes[1..] => {
                            // Found the correct v
                            break;
                        }
                        Ok(_) => {
                            // Found a key, but not the correct one
                            // Try the other v value
                            v = 1;
                            continue;
                        }
                        Err(_) if v == 1 => {
                            // We tried both v values, but no key was found
                            // This should never happen, but if it does, we will just
                            // leave v as 1 and break
                            break;
                        }
                        Err(_) => {
                            // No key was found, try the other v value
                            v = 1;
                            continue;
                        }
                    }
                }
                signature_bytes[64] = v + 27;

                let job_result = JobResult::DKGPhaseTwo(DKGTSSSignatureResult {
                    signature_scheme: DigitalSignatureScheme::Ecdsa,
                    data: additional_params.input_data_to_sign.try_into().unwrap(),
                    signature: signature_bytes.to_vec().try_into().unwrap(),
                    verifying_key: public_key_bytes.try_into().unwrap(),
                });

                client
                    .submit_job_result(
                        additional_params.role_type,
                        additional_params.job_id,
                        job_result,
                    )
                    .await
                    .map_err(|err| JobError {
                        reason: format!("Failed to submit job result: {err:?}"),
                    })?;
            }

            Ok(())
        })
        .build())
}
