use dfns_cggmp21::signing::msg::Msg;
use dfns_cggmp21::supported_curves::Secp256k1;
use dfns_cggmp21::KeyShare;
use gadget_common::client::{ClientWithApi, JobsApiForGadget};
use gadget_common::gadget::message::{GadgetProtocolMessage, UserID};
use gadget_common::gadget::network::Network;
use gadget_common::gadget::work_manager::WorkManager;
use gadget_common::gadget::JobInitMetadata;
use gadget_common::keystore::KeystoreBackend;
use gadget_common::prelude::FullProtocolConfig;
use gadget_common::utils::CloneableUnboundedReceiver;
use gadget_common::Block;
use gadget_core::job::{BuiltExecutableJobWrapper, JobBuilder, JobError};
use gadget_core::job_manager::{ProtocolWorkManager, WorkManagerInterface};
use rand::SeedableRng;
use round_based_21::{Incoming, Outgoing};
use sc_client_api::Backend;
use sha2::Sha256;
use sp_api::ProvideRuntimeApi;
use sp_core::{ecdsa, keccak_256, Pair};
use std::collections::HashMap;
use std::sync::Arc;
use tangle_primitives::jobs::{
    DKGTSSKeyRotationResult, DigitalSignatureScheme, JobId, JobResult, JobType,
};
use tangle_primitives::roles::RoleType;
use tokio::sync::mpsc::UnboundedReceiver;

pub async fn create_next_job<
    B: Block,
    BE: Backend<B> + 'static,
    C: ClientWithApi<B, BE>,
    KBE: KeystoreBackend,
    N: Network,
>(
    config: &crate::DfnsKeyRotateProtocol<B, BE, C, N, KBE>,
    job: JobInitMetadata<B>,
    _work_manager: &ProtocolWorkManager<WorkManager>,
) -> Result<DfnsCGGMP21KeyRotateExtraParams, gadget_common::Error>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    let job_id = job.job_id;

    let JobType::DKGTSSPhaseFour(p4_job) = job.job_type else {
        panic!("Should be valid type")
    };
    let phase_one_id = p4_job.phase_one_id;
    let new_phase_one_id = p4_job.new_phase_one_id;

    let phase1_job = job.phase1_job.expect("Should exist for a phase 2 job");
    let t = phase1_job.get_threshold().expect("Should exist") as u16;

    let seed = keccak_256(&[&job_id.to_be_bytes()[..], &job.retry_id.to_be_bytes()[..]].concat());
    let mut rng = rand_chacha::ChaChaRng::from_seed(seed);

    let (i, signers, mapping) = super::util::choose_signers(
        &mut rng,
        &config.key_store.pair().public(),
        &job.participants_role_ids,
        t,
    )?;

    let new_phase_one_result = config
        .get_jobs_client()
        .query_job_result(job.at, job.role_type, new_phase_one_id)
        .await?
        .ok_or_else(|| gadget_common::Error::ClientError {
            err: format!("No key found for job ID: {new_phase_one_id}"),
        })?;

    let new_key = match new_phase_one_result.result {
        JobResult::DKGPhaseOne(r) => r.key,
        _ => {
            return Err(gadget_common::Error::ClientError {
                err: format!("Wrong job result type for job ID: {new_phase_one_id}"),
            })
        }
    };

    let key = config
        .key_store
        .get_job_result(phase_one_id)
        .await
        .map_err(|err| gadget_common::Error::ClientError {
            err: err.to_string(),
        })?
        .ok_or_else(|| gadget_common::Error::ClientError {
            err: format!("No key found for job ID: {job_id:?}"),
        })?;

    let user_id_to_account_id_mapping = Arc::new(mapping);

    let params = DfnsCGGMP21KeyRotateExtraParams {
        i,
        t,
        signers,
        job_id,
        phase_one_id,
        new_phase_one_id,
        role_type: job.role_type,
        key,
        new_key: new_key.into(),
        user_id_to_account_id_mapping,
    };
    Ok(params)
}

#[derive(Clone)]
pub struct DfnsCGGMP21KeyRotateExtraParams {
    i: u16,
    t: u16,
    signers: Vec<u16>,
    job_id: JobId,
    phase_one_id: JobId,
    new_phase_one_id: JobId,
    role_type: RoleType,
    key: KeyShare<Secp256k1>,
    new_key: Vec<u8>,
    user_id_to_account_id_mapping: Arc<HashMap<UserID, ecdsa::Public>>,
}

pub async fn generate_protocol_from<
    B: Block,
    BE: Backend<B> + 'static,
    C: ClientWithApi<B, BE>,
    KBE: KeystoreBackend,
    N: Network,
>(
    config: &crate::DfnsKeyRotateProtocol<B, BE, C, N, KBE>,
    associated_block_id: <WorkManager as WorkManagerInterface>::Clock,
    associated_retry_id: <WorkManager as WorkManagerInterface>::RetryID,
    associated_session_id: <WorkManager as WorkManagerInterface>::SessionID,
    associated_task_id: <WorkManager as WorkManagerInterface>::TaskID,
    protocol_message_channel: UnboundedReceiver<GadgetProtocolMessage>,
    additional_params: DfnsCGGMP21KeyRotateExtraParams,
) -> Result<BuiltExecutableJobWrapper, JobError>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    let debug_logger_post = config.logger.clone();
    let logger = debug_logger_post.clone();
    let protocol_output = Arc::new(tokio::sync::Mutex::new(None));
    let protocol_output_clone = protocol_output.clone();
    let client = config.get_jobs_client();
    let role_id = config.key_store.pair().public();
    let phase_one_id = additional_params.phase_one_id;
    let network = config.clone();

    let (i, signers, t, new_phase_one_id, key, new_key, mapping) = (
        additional_params.i,
        additional_params.signers,
        additional_params.t,
        additional_params.new_phase_one_id,
        additional_params.key,
        additional_params.new_key.clone(),
        additional_params.user_id_to_account_id_mapping.clone(),
    );

    let public_key_bytes = key.shared_public_key().to_bytes(true).to_vec();
    let new_key2 = new_key.clone();

    Ok(JobBuilder::new()
        .protocol(async move {
            let mut rng = rand::rngs::StdRng::from_entropy();
            let protocol_message_channel =
                CloneableUnboundedReceiver::from(protocol_message_channel);

            logger.info(format!(
                "Starting Key Rotation Protocol with params: i={i}, t={t}"
            ));

            let job_id_bytes = additional_params.job_id.to_be_bytes();
            let mix = keccak_256(b"dnfs-cggmp21-key-rotate");
            let eid_bytes = [&job_id_bytes[..], &mix[..]].concat();
            let eid = dfns_cggmp21::ExecutionId::new(&eid_bytes);
            let (
                key_rotate_tx_to_outbound,
                key_rotate_rx_async_proto,
                _broadcast_tx_to_outbound,
                _broadcast_rx_from_gadget,
            ) = gadget_common::channels::create_job_manager_to_async_protocol_channel_split_io::<
                _,
                (),
                Outgoing<Msg<Secp256k1, Sha256>>,
                Incoming<Msg<Secp256k1, Sha256>>,
            >(
                protocol_message_channel.clone(),
                associated_block_id,
                associated_retry_id,
                associated_session_id,
                associated_task_id,
                mapping.clone(),
                role_id,
                network.clone(),
            );

            let delivery = (key_rotate_rx_async_proto, key_rotate_tx_to_outbound);
            let party = round_based_21::MpcParty::connected(delivery);
            let data_hash = keccak_256(&new_key);
            let data_to_sign = dfns_cggmp21::DataToSign::from_scalar(
                dfns_cggmp21::generic_ec::Scalar::from_be_bytes_mod_order(data_hash),
            );
            let signature = dfns_cggmp21::signing(eid, i, &signers, &key)
                .sign(&mut rng, party, data_to_sign)
                .await
                .map_err(|err| JobError {
                    reason: format!("Key Rotation protocol error: {err:?}"),
                })?;

            // Normalize the signature
            let signature = signature.normalize_s();
            logger.debug("Finished AsyncProtocol - Key Rotation");
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
                    let data_hash = keccak_256(&new_key2);
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
                // Add 27 to the recovery ID
                signature_bytes[64] = v + 27;

                let job_result = JobResult::DKGPhaseFour(DKGTSSKeyRotationResult {
                    signature_scheme: DigitalSignatureScheme::Ecdsa,
                    signature: signature_bytes.to_vec().try_into().unwrap(),
                    phase_one_id,
                    new_phase_one_id,
                    new_key: new_key2.try_into().unwrap(),
                    key: public_key_bytes.try_into().unwrap(),
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
