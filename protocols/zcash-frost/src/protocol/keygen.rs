use frost_ed25519::Ed25519Sha512;
use frost_ed448::Ed448Shake256;
use frost_p256::P256Sha256;
use frost_p384::P384Sha384;
use frost_ristretto255::Ristretto255Sha512;
use frost_secp256k1::Secp256K1Sha256;
use futures::StreamExt;
use gadget_common::client::JobsApiForGadget;
use gadget_common::client::{
    ClientWithApi, GadgetJobResult, MaxKeyLen, MaxParticipants, MaxSignatureLen,
};
use gadget_common::config::{Network, ProvideRuntimeApi};
use gadget_common::debug_logger::DebugLogger;
use gadget_common::gadget::message::{GadgetProtocolMessage, UserID};
use gadget_common::gadget::work_manager::WorkManager;
use gadget_common::gadget::JobInitMetadata;
use gadget_common::keystore::{ECDSAKeyStore, KeystoreBackend};
use gadget_common::{channels, Block};
use gadget_core::job::{BuiltExecutableJobWrapper, JobBuilder, JobError};
use gadget_core::job_manager::{ProtocolWorkManager, WorkManagerInterface};
use itertools::Itertools;
use pallet_dkg::signatures_schemes::ecdsa::verify_signer_from_set_ecdsa;
use pallet_dkg::signatures_schemes::to_slice_33;
use rand::SeedableRng;
use round_based_21::{Incoming, Outgoing};
use sc_client_api::Backend;
use sp_application_crypto::sp_core::keccak_256;
use sp_core::{ecdsa, ByteArray, Pair};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tangle_primitives::jobs::{DKGTSSKeySubmissionResult, DigitalSignatureScheme, JobId, JobType};
use tangle_primitives::roles::{RoleType, ThresholdSignatureRoleType};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::rounds;
use crate::rounds::keygen::Msg;

use crate::protocol::util::account_id_32_to_public;
use gadget_common::channels::PublicKeyGossipMessage;

#[derive(Clone)]
pub struct ZcashFrostKeygenExtraParams {
    pub i: u16,
    pub t: u16,
    pub n: u16,
    pub job_id: JobId,
    pub role_type: RoleType,
    pub user_id_to_account_id_mapping: Arc<HashMap<UserID, ecdsa::Public>>,
}

pub async fn create_next_job<
    B: Block,
    BE: Backend<B>,
    C: ClientWithApi<B, BE>,
    N: Network,
    KBE: KeystoreBackend,
>(
    config: &crate::ZcashFrostKeygenProtocol<B, BE, C, N, KBE>,
    job: JobInitMetadata<B>,
    _work_manager: &ProtocolWorkManager<WorkManager>,
) -> Result<ZcashFrostKeygenExtraParams, gadget_common::Error>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    let job_id = job.job_id;
    let role_type = job.job_type.get_role_type();

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
            .map(|r| {
                (
                    r.0 as UserID,
                    account_id_32_to_public(r.1.as_slice()).expect("Should convert"),
                )
            })
            .collect(),
    );

    let params = ZcashFrostKeygenExtraParams {
        i: participants
            .iter()
            .position(|p| p == &config.account_id)
            .expect("Should exist") as u16,
        t: threshold as u16,
        n: participants.len() as u16,
        role_type,
        job_id,
        user_id_to_account_id_mapping,
    };

    Ok(params)
}

macro_rules! run_threshold_keygen {
    ($impl_type:ty, $tracer:expr, $i:expr, $t:expr, $n:expr, $role:expr, $rng:expr, $party:expr) => {
        rounds::keygen::run_threshold_keygen::<$impl_type, _, _>(
            Some($tracer),
            $i,
            $t,
            $n,
            $role,
            $rng,
            $party,
        )
        .await
        .map_err(|err| {
            println!("Keygen protocol error: {err:#?}");
            err.to_string()
        })?
    };
}

pub async fn generate_protocol_from<
    B: Block,
    BE: Backend<B>,
    C: ClientWithApi<B, BE>,
    N: Network,
    KBE: KeystoreBackend,
>(
    config: &crate::ZcashFrostKeygenProtocol<B, BE, C, N, KBE>,
    associated_block_id: <WorkManager as WorkManagerInterface>::Clock,
    associated_retry_id: <WorkManager as WorkManagerInterface>::RetryID,
    associated_session_id: <WorkManager as WorkManagerInterface>::SessionID,
    associated_task_id: <WorkManager as WorkManagerInterface>::TaskID,
    protocol_message_channel: UnboundedReceiver<GadgetProtocolMessage>,
    additional_params: ZcashFrostKeygenExtraParams,
) -> Result<BuiltExecutableJobWrapper, JobError>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    let key_store = config.key_store.clone();
    let key_store2 = config.key_store.clone();
    let protocol_output = Arc::new(tokio::sync::Mutex::new(None));
    let protocol_output_clone = protocol_output.clone();
    let pallet_tx = config.pallet_tx.clone();
    let id = account_id_32_to_public(config.account_id.as_slice()).expect("Should convert");
    let logger = config.logger.clone();
    let network = config.clone();

    let (i, t, n, mapping, role_type) = (
        additional_params.i,
        additional_params.t,
        additional_params.n,
        additional_params.user_id_to_account_id_mapping,
        additional_params.role_type,
    );

    let role = match role_type {
        RoleType::Tss(role) => role,
        _ => {
            return Err(JobError {
                reason: "Invalid role type".to_string(),
            })
        }
    };

    Ok(JobBuilder::new()
        .protocol(async move {
            let mut rng = rand::rngs::StdRng::from_entropy();
            let protocol_message_channel =
                gadget_common::utils::CloneableUnboundedReceiver::from(protocol_message_channel);
            logger.info(format!(
                "Starting Keygen Protocol with params: i={i}, t={t}, n={n}"
            ));

            let (
                keygen_tx_to_outbound,
                keygen_rx_async_proto,
                broadcast_tx_to_outbound,
                broadcast_rx_from_gadget,
            ) = channels::create_job_manager_to_async_protocol_channel_split_io::<
                _,
                _,
                Outgoing<Msg>,
                Incoming<Msg>,
            >(
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
            let delivery = (keygen_rx_async_proto, keygen_tx_to_outbound);
            let party = round_based_21::MpcParty::connected(delivery);
            let frost_key_share_package = match role {
                ThresholdSignatureRoleType::ZcashFrostEd25519 => {
                    run_threshold_keygen!(
                        Ed25519Sha512,
                        &mut tracer,
                        i,
                        t,
                        n,
                        role,
                        &mut rng,
                        party
                    )
                }
                ThresholdSignatureRoleType::ZcashFrostEd448 => {
                    run_threshold_keygen!(
                        Ed448Shake256,
                        &mut tracer,
                        i,
                        t,
                        n,
                        role,
                        &mut rng,
                        party
                    )
                }
                ThresholdSignatureRoleType::ZcashFrostP256 => {
                    run_threshold_keygen!(P256Sha256, &mut tracer, i, t, n, role, &mut rng, party)
                }
                ThresholdSignatureRoleType::ZcashFrostP384 => {
                    run_threshold_keygen!(P384Sha384, &mut tracer, i, t, n, role, &mut rng, party)
                }
                ThresholdSignatureRoleType::ZcashFrostRistretto255 => {
                    run_threshold_keygen!(
                        Ristretto255Sha512,
                        &mut tracer,
                        i,
                        t,
                        n,
                        role,
                        &mut rng,
                        party
                    )
                }
                ThresholdSignatureRoleType::ZcashFrostSecp256k1 => {
                    run_threshold_keygen!(
                        Secp256K1Sha256,
                        &mut tracer,
                        i,
                        t,
                        n,
                        role,
                        &mut rng,
                        party
                    )
                }
                _ => unreachable!("Invalid role"),
            };
            let perf_report = tracer.get_report().map_err(|err| JobError {
                reason: format!("Keygen protocol error: {err:?}"),
            })?;
            logger.trace(format!("Incomplete Keygen protocol report: {perf_report}"));
            logger.debug("Finished AsyncProtocol - Incomplete Keygen");

            let job_result = handle_public_key_gossip(
                key_store2,
                &logger,
                &frost_key_share_package.verifying_key,
                role,
                t,
                i,
                broadcast_tx_to_outbound,
                broadcast_rx_from_gadget,
            )
            .await?;

            *protocol_output.lock().await = Some((frost_key_share_package, job_result));
            Ok(())
        })
        .post(async move {
            // TODO: handle protocol blames
            // Store the keys locally, as well as submitting them to the blockchain
            if let Some((local_key, job_result)) = protocol_output_clone.lock().await.take() {
                key_store
                    .set_job_result(additional_params.job_id, local_key)
                    .await
                    .map_err(|err| JobError {
                        reason: format!("Failed to store key: {err:?}"),
                    })?;

                pallet_tx
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

#[allow(clippy::too_many_arguments)]
async fn handle_public_key_gossip<KBE: KeystoreBackend>(
    key_store: ECDSAKeyStore<KBE>,
    logger: &DebugLogger,
    public_key_package: &[u8],
    role: ThresholdSignatureRoleType,
    t: u16,
    i: u16,
    broadcast_tx_to_outbound: futures::channel::mpsc::UnboundedSender<PublicKeyGossipMessage>,
    mut broadcast_rx_from_gadget: futures::channel::mpsc::UnboundedReceiver<PublicKeyGossipMessage>,
) -> Result<GadgetJobResult, JobError> {
    let key_hashed = keccak_256(public_key_package);
    let signature = key_store.pair().sign_prehashed(&key_hashed).0.to_vec();
    let my_id = key_store.pair().public();
    let mut received_keys = BTreeMap::new();
    received_keys.insert(i, signature.clone());
    let mut received_participants = BTreeMap::new();
    received_participants.insert(i, my_id);

    broadcast_tx_to_outbound
        .unbounded_send(PublicKeyGossipMessage {
            from: i as _,
            to: None,
            signature,
            id: my_id,
        })
        .map_err(|err| JobError {
            reason: format!("Failed to send public key: {err:?}"),
        })?;

    for _ in 0..t {
        let message = broadcast_rx_from_gadget
            .next()
            .await
            .ok_or_else(|| JobError {
                reason: "Failed to receive public key".to_string(),
            })?;

        let from = message.from;
        logger.debug(format!("Received public key from {from}"));

        if received_keys.contains_key(&(from as u16)) {
            logger.warn("Received duplicate key");
            continue;
        }
        // verify signature
        let maybe_signature = sp_core::ecdsa::Signature::from_slice(&message.signature);
        match maybe_signature.and_then(|s| s.recover_prehashed(&key_hashed)) {
            Some(p) if p != message.id => {
                logger.warn(format!(
                    "Received invalid signature from {from} not signed by them"
                ));
            }
            Some(p) if p == message.id => {
                logger.debug(format!("Received valid signature from {from}"));
            }
            Some(_) => unreachable!("Should not happen"),
            None => {
                logger.warn(format!("Received invalid signature from {from}"));
                continue;
            }
        }

        received_keys.insert(from as u16, message.signature);
        received_participants.insert(from as u16, message.id);
        logger.debug(format!(
            "Received {}/{} signatures",
            received_keys.len(),
            t + 1
        ));
    }

    // Order and collect the map to ensure symmetric submission to blockchain
    let signatures = received_keys
        .into_iter()
        .sorted_by_key(|x| x.0)
        .map(|r| r.1.try_into().unwrap())
        .collect::<Vec<_>>();

    let participants = received_participants
        .into_iter()
        .sorted_by_key(|x| x.0)
        .map(|r| r.1 .0.to_vec().try_into().unwrap())
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    if signatures.len() < t as usize {
        return Err(JobError {
            reason: format!(
                "Received {} signatures, expected at least {}",
                signatures.len(),
                t + 1,
            ),
        });
    }

    let res = DKGTSSKeySubmissionResult {
        signature_scheme: match role {
            ThresholdSignatureRoleType::ZcashFrostEd25519 => DigitalSignatureScheme::SchnorrEd25519,
            ThresholdSignatureRoleType::ZcashFrostEd448 => DigitalSignatureScheme::SchnorrEd448,
            ThresholdSignatureRoleType::ZcashFrostP256 => DigitalSignatureScheme::SchnorrP256,
            ThresholdSignatureRoleType::ZcashFrostP384 => DigitalSignatureScheme::SchnorrP384,
            ThresholdSignatureRoleType::ZcashFrostRistretto255 => {
                DigitalSignatureScheme::SchnorrRistretto255
            }
            ThresholdSignatureRoleType::ZcashFrostSecp256k1 => {
                DigitalSignatureScheme::SchnorrSecp256k1
            }
            _ => unreachable!("Invalid role"),
        },
        key: public_key_package.to_vec().try_into().unwrap(),
        participants,
        signatures: signatures.try_into().unwrap(),
        threshold: t as _,
    };
    verify_generated_dkg_key_ecdsa(res.clone(), logger);
    Ok(GadgetJobResult::DKGPhaseOne(res))
}

fn verify_generated_dkg_key_ecdsa(
    data: DKGTSSKeySubmissionResult<MaxKeyLen, MaxParticipants, MaxSignatureLen>,
    logger: &DebugLogger,
) {
    // Ensure participants and signatures are not empty
    assert!(!data.participants.is_empty(), "NoParticipantsFound",);
    assert!(!data.signatures.is_empty(), "NoSignaturesFound");

    // Generate the required ECDSA signers
    let maybe_signers = data
        .participants
        .iter()
        .map(|x| {
            ecdsa::Public(
                to_slice_33(x)
                    .unwrap_or_else(|| panic!("Failed to convert input to ecdsa public key")),
            )
        })
        .collect::<Vec<ecdsa::Public>>();

    assert!(!maybe_signers.is_empty(), "NoParticipantsFound");

    let mut known_signers: Vec<ecdsa::Public> = Default::default();

    for signature in data.signatures {
        // Ensure the required signer signature exists
        let (maybe_authority, success) =
            verify_signer_from_set_ecdsa(maybe_signers.clone(), &data.key, &signature);

        if success {
            let authority = maybe_authority.expect("CannotRetreiveSigner");

            // Ensure no duplicate signatures
            assert!(!known_signers.contains(&authority), "DuplicateSignature");

            logger.debug(format!("Verified signature from {}", authority));
            known_signers.push(authority);
        }
    }

    // Ensure a sufficient number of unique signers are present
    assert!(
        known_signers.len() > data.threshold as usize,
        "NotEnoughSigners"
    );
    logger.debug(format!(
        "Verified {}/{} signatures",
        known_signers.len(),
        data.threshold + 1
    ));
}
