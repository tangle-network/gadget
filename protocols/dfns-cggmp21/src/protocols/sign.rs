use dfns_cggmp21::generic_ec::coords::HasAffineX;
use dfns_cggmp21::generic_ec::{Curve, Point};
use dfns_cggmp21::round_based::{Delivery, MpcParty};
use dfns_cggmp21::security_level::SecurityLevel;
use dfns_cggmp21::signing::msg::Msg;

use dfns_cggmp21::supported_curves::{Secp256k1, Secp256r1, Stark};
use dfns_cggmp21::{DataToSign, KeyShare};
use gadget_common::client::{ClientWithApi, JobsApiForGadget};
use gadget_common::config::DebugLogger;
use gadget_common::gadget::message::{GadgetProtocolMessage, UserID};
use gadget_common::gadget::work_manager::WorkManager;
use gadget_common::gadget::JobInitMetadata;
use gadget_common::keystore::KeystoreBackend;
use gadget_common::prelude::{FullProtocolConfig, Network};
use gadget_common::Block;
use gadget_core::job::{BuiltExecutableJobWrapper, JobBuilder, JobError};
use gadget_core::job_manager::{ProtocolWorkManager, WorkManagerInterface};
use rand::{CryptoRng, RngCore, SeedableRng};

use sc_client_api::Backend;

use crate::protocols::{DefaultCryptoHasher, DefaultSecurityLevel};
use dfns_cggmp21::signing::SigningBuilder;
use digest::typenum::U32;
use digest::Digest;
use sp_api::ProvideRuntimeApi;
use sp_core::{ecdsa, keccak_256, Pair};
use std::collections::HashMap;
use std::sync::Arc;
use tangle_primitives::jobs::{
    DKGTSSSignatureResult, DigitalSignatureScheme, JobId, JobResult, JobType,
};
use tangle_primitives::roles::{RoleType, ThresholdSignatureRoleType};
use tokio::sync::mpsc::UnboundedReceiver;

use super::keygen::create_party;

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
    let t = phase1_job.get_threshold().expect("Should exist") as u16;

    let seed = keccak_256(&[&job_id.to_be_bytes()[..], &job.retry_id.to_be_bytes()[..]].concat());
    let mut rng = rand_chacha::ChaChaRng::from_seed(seed);

    let (i, signers, mapping) = super::util::choose_signers(
        &mut rng,
        &config.key_store.pair().public(),
        &job.participants_role_ids,
        t,
    )?;
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
    key: Vec<u8>,
    input_data_to_sign: Vec<u8>,
    user_id_to_account_id_mapping: Arc<HashMap<UserID, ecdsa::Public>>,
}

#[allow(clippy::too_many_arguments)]
pub async fn run_and_serialize_signing<'r, E, L, R, D, H>(
    logger: &DebugLogger,
    tracer: &mut dfns_cggmp21::progress::PerfProfiler,
    eid: dfns_cggmp21::ExecutionId<'r>,
    i: u16,
    signers: Vec<u16>,
    msg: DataToSign<E>,
    key: Vec<u8>,
    party: MpcParty<Msg<E, H>, D>,
    rng: &mut R,
) -> Result<Vec<u8>, JobError>
where
    E: Curve,
    Point<E>: HasAffineX<E>,
    L: SecurityLevel,
    R: RngCore + CryptoRng,
    D: Delivery<Msg<E, H>>,
    H: Digest<OutputSize = U32> + Clone + Send + 'static,
{
    let key: KeyShare<E, L> = bincode2::deserialize(&key).map_err(|err| JobError {
        reason: format!("Keygen protocol error: {err:?}"),
    })?;
    let signing_builder = SigningBuilder::<E, L, H>::new(eid, i, &signers, &key);
    let signature = signing_builder
        .set_progress_tracer(tracer)
        .sign(rng, party, msg)
        .await
        .map_err(|err| JobError {
            reason: format!("Signing protocol error: {err:?}"),
        })?;

    let perf_report = tracer.get_report().map_err(|err| JobError {
        reason: format!("Signing protocol error: {err:?}"),
    })?;
    logger.trace(format!("Signing protocol report: {perf_report}"));
    // Normalize the signature
    bincode2::serialize(&signature.normalize_s()).map_err(|err| JobError {
        reason: format!("Signing protocol error: {err:?}"),
    })
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
    let my_role_id = config.key_store.pair().public();
    let network = config.clone();

    let (i, signers, t, local_key_serialized, role_type, input_data_to_sign, mapping) = (
        additional_params.i,
        additional_params.signers,
        additional_params.t,
        additional_params.key,
        additional_params.role_type,
        additional_params.input_data_to_sign.clone(),
        additional_params.user_id_to_account_id_mapping.clone(),
    );

    let public_key = match role_type {
        RoleType::Tss(ThresholdSignatureRoleType::DfnsCGGMP21Secp256k1) => {
            get_public_key_from_serialized_local_key_bytes::<Secp256k1>(&local_key_serialized)?
        }
        RoleType::Tss(ThresholdSignatureRoleType::DfnsCGGMP21Secp256r1) => {
            get_public_key_from_serialized_local_key_bytes::<Secp256r1>(&local_key_serialized)?
        }
        RoleType::Tss(ThresholdSignatureRoleType::DfnsCGGMP21Stark) => {
            get_public_key_from_serialized_local_key_bytes::<Stark>(&local_key_serialized)?
        }
        _ => {
            return Err(JobError {
                reason: format!("Invalid role type: {role_type:?}"),
            });
        }
    };

    Ok(JobBuilder::new()
        .protocol(async move {
            let mut rng = rand::rngs::StdRng::from_entropy();
            logger.info(format!(
                "Starting Signing Protocol with params: i={i}, t={t}"
            ));

            let job_id_bytes = additional_params.job_id.to_be_bytes();
            let mix = keccak_256(b"dnfs-cggmp21-signing");
            let eid_bytes = [&job_id_bytes[..], &mix[..]].concat();
            let eid = dfns_cggmp21::ExecutionId::new(&eid_bytes);

            let mut tracer = dfns_cggmp21::progress::PerfProfiler::new();
            let data_hash = keccak_256(&input_data_to_sign);
            let data_to_sign = dfns_cggmp21::DataToSign::from_scalar(
                dfns_cggmp21::generic_ec::Scalar::from_be_bytes_mod_order(data_hash),
            );
            let signature = match role_type {
                RoleType::Tss(ThresholdSignatureRoleType::DfnsCGGMP21Secp256k1) => {
                    let (_, _, party) = create_party::<
                        Secp256k1,
                        _,
                        DefaultSecurityLevel,
                        Msg<Secp256k1, DefaultCryptoHasher>,
                    >(
                        protocol_message_channel,
                        associated_block_id,
                        associated_retry_id,
                        associated_session_id,
                        associated_task_id,
                        mapping.clone(),
                        my_role_id,
                        network.clone(),
                    );
                    run_and_serialize_signing::<_, DefaultSecurityLevel, _, _, DefaultCryptoHasher>(
                        &logger,
                        &mut tracer,
                        eid,
                        i,
                        signers,
                        data_to_sign,
                        local_key_serialized,
                        party,
                        &mut rng,
                    )
                    .await?
                }
                RoleType::Tss(ThresholdSignatureRoleType::DfnsCGGMP21Secp256r1) => {
                    let (_, _, party) = create_party::<
                        Secp256r1,
                        _,
                        DefaultSecurityLevel,
                        Msg<Secp256k1, DefaultCryptoHasher>,
                    >(
                        protocol_message_channel,
                        associated_block_id,
                        associated_retry_id,
                        associated_session_id,
                        associated_task_id,
                        mapping.clone(),
                        my_role_id,
                        network.clone(),
                    );
                    run_and_serialize_signing::<_, DefaultSecurityLevel, _, _, DefaultCryptoHasher>(
                        &logger,
                        &mut tracer,
                        eid,
                        i,
                        signers,
                        data_to_sign,
                        local_key_serialized,
                        party,
                        &mut rng,
                    )
                    .await?
                }
                RoleType::Tss(ThresholdSignatureRoleType::DfnsCGGMP21Stark) => {
                    let (_, _, party) = create_party::<
                        Stark,
                        _,
                        DefaultSecurityLevel,
                        Msg<Secp256k1, DefaultCryptoHasher>,
                    >(
                        protocol_message_channel,
                        associated_block_id,
                        associated_retry_id,
                        associated_session_id,
                        associated_task_id,
                        mapping.clone(),
                        my_role_id,
                        network.clone(),
                    );
                    run_and_serialize_signing::<_, DefaultSecurityLevel, _, _, DefaultCryptoHasher>(
                        &logger,
                        &mut tracer,
                        eid,
                        i,
                        signers,
                        data_to_sign,
                        local_key_serialized,
                        party,
                        &mut rng,
                    )
                    .await?
                }
                _ => {
                    return Err(JobError {
                        reason: format!("Invalid role type: {role_type:?}"),
                    });
                }
            };

            logger.debug("Finished AsyncProtocol - Signing");
            *protocol_output.lock().await = Some(signature);
            Ok(())
        })
        .post(async move {
            // Submit the protocol output to the blockchain
            if let Some(signature) = protocol_output_clone.lock().await.take() {
                let job_result = JobResult::DKGPhaseTwo(DKGTSSSignatureResult {
                    signature_scheme: DigitalSignatureScheme::Ecdsa,
                    data: additional_params.input_data_to_sign.try_into().unwrap(),
                    signature: signature.try_into().unwrap(),
                    verifying_key: public_key.try_into().unwrap(),
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

fn get_public_key_from_serialized_local_key_bytes<E: Curve>(
    local_key_serialized: &[u8],
) -> Result<Vec<u8>, JobError> {
    let key_share = bincode2::deserialize::<KeyShare<E, DefaultSecurityLevel>>(
        local_key_serialized,
    )
    .map_err(|err| JobError {
        reason: format!("Keygen protocol error: {err:?}"),
    })?;
    Ok(key_share.shared_public_key().to_bytes(true).to_vec())
}
