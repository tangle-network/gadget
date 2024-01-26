use async_trait::async_trait;
use dfns_cggmp21::supported_curves::Secp256k1;
use dfns_cggmp21::KeyShare;
use futures::StreamExt;
use gadget_common::client::{AccountId, ClientWithApi, JobsClient};
use gadget_common::debug_logger::DebugLogger;
use gadget_common::gadget::message::{GadgetProtocolMessage, UserID};
use gadget_common::gadget::network::Network;
use gadget_common::gadget::work_manager::WorkManager;
use gadget_common::gadget::{GadgetProtocol, JobInitMetadata, WorkManagerConfig};
use gadget_common::keystore::{ECDSAKeyStore, KeystoreBackend};
use gadget_common::protocol::AsyncProtocol;
use gadget_common::{Block, BlockImportNotification};
use gadget_core::job::{BuiltExecutableJobWrapper, JobBuilder, JobError};
use gadget_core::job_manager::{ProtocolWorkManager, WorkManagerInterface};
use itertools::Itertools;
use pallet_jobs_rpc_runtime_api::JobsApi;
use rand::SeedableRng;
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_application_crypto::sp_core::keccak_256;
use sp_core::{ecdsa, Pair};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tangle_primitives::jobs::{
    DKGTSSKeySubmissionResult, DigitalSignatureType, JobId, JobResult, JobType,
};
use tangle_primitives::roles::{RoleType, ThresholdSignatureRoleType};
use tokio::sync::mpsc::UnboundedReceiver;

use super::util::PublicKeyGossipMessage;

pub struct DfnsCGGMP21KeygenProtocol<B: Block, BE, KBE: KeystoreBackend, C, N> {
    client: JobsClient<B, BE, C>,
    key_store: ECDSAKeyStore<KBE>,
    network: N,
    logger: DebugLogger,
    account_id: AccountId,
}

pub async fn create_protocol<B, BE, KBE, C, N>(
    account_id: AccountId,
    client: JobsClient<B, BE, C>,
    network: N,
    logger: DebugLogger,
    key_store: ECDSAKeyStore<KBE>,
) -> DfnsCGGMP21KeygenProtocol<B, BE, KBE, C, N>
where
    B: Block,
    BE: Backend<B>,
    C: ClientWithApi<B, BE>,
    KBE: KeystoreBackend,
    N: Network,
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    DfnsCGGMP21KeygenProtocol {
        client,
        network,
        key_store,
        logger,
        account_id,
    }
}

#[async_trait]
impl<
        B: Block,
        BE: Backend<B> + 'static,
        C: ClientWithApi<B, BE>,
        KBE: KeystoreBackend,
        N: Network,
    > GadgetProtocol<B, BE, C> for DfnsCGGMP21KeygenProtocol<B, BE, KBE, C, N>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    async fn create_next_job(
        &self,
        job: JobInitMetadata,
    ) -> Result<<Self as AsyncProtocol>::AdditionalParams, gadget_common::Error> {
        let now = job.now;
        self.logger.info(format!("At finality notification {now}"));

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
                .map(|r| (r.0 as UserID, r.1))
                .collect(),
        );

        let params = DfnsCGGMP21KeygenExtraParams {
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
    ) -> Result<(), gadget_common::Error> {
        Ok(())
    }

    async fn process_error(
        &self,
        error: gadget_common::Error,
        _job_manager: &ProtocolWorkManager<WorkManager>,
    ) {
        log::error!(target: "dfns_cggmp1", "Error: {error:?}");
    }

    fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    fn role_filter(&self, role: RoleType) -> bool {
        matches!(role, RoleType::Tss(ThresholdSignatureRoleType::DfnsCGGMP21Secp256k1))
    }

    fn phase_filter(&self, job: JobType<AccountId>) -> bool {
        matches!(job, JobType::DKGTSSPhaseOne(_))
    }


    fn client(&self) -> &JobsClient<B, BE, C> {
        &self.client
    }

    fn logger(&self) -> &DebugLogger {
        &self.logger
    }

    fn get_work_manager_config(&self) -> WorkManagerConfig {
        WorkManagerConfig {
            interval: None, // Manual polling
            max_active_tasks: crate::constants::keygen_worker::MAX_RUNNING_TASKS,
            max_pending_tasks: crate::constants::keygen_worker::MAX_ENQUEUED_TASKS,
        }
    }
}

pub struct DfnsCGGMP21KeygenExtraParams {
    i: u16,
    t: u16,
    n: u16,
    job_id: JobId,
    role_type: RoleType,
    user_id_to_account_id_mapping: Arc<HashMap<UserID, AccountId>>,
}

#[async_trait]
impl<
        B: Block,
        BE: Backend<B> + 'static,
        KBE: KeystoreBackend,
        C: ClientWithApi<B, BE>,
        N: Network,
    > AsyncProtocol for DfnsCGGMP21KeygenProtocol<B, BE, KBE, C, N>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    type AdditionalParams = DfnsCGGMP21KeygenExtraParams;
    async fn generate_protocol_from(
        &self,
        associated_block_id: <WorkManager as WorkManagerInterface>::Clock,
        associated_retry_id: <WorkManager as WorkManagerInterface>::RetryID,
        associated_session_id: <WorkManager as WorkManagerInterface>::SessionID,
        associated_task_id: <WorkManager as WorkManagerInterface>::TaskID,
        protocol_message_channel: UnboundedReceiver<GadgetProtocolMessage>,
        additional_params: Self::AdditionalParams,
    ) -> Result<BuiltExecutableJobWrapper, JobError> {
        let key_store = self.key_store.clone();
        let key_store_for_gossip = self.key_store.clone();
        let protocol_output = Arc::new(tokio::sync::Mutex::new(None));
        let protocol_output_clone = protocol_output.clone();
        let client = self.client.clone();
        let id = self.account_id;
        let logger = self.logger.clone();
        let network = self.network.clone();

        let (i, t, n, mapping) = (
            additional_params.i,
            additional_params.t,
            additional_params.n,
            additional_params.user_id_to_account_id_mapping,
        );

        Ok(JobBuilder::new()
            .protocol(async move {
                let mut rng = rand::rngs::StdRng::from_entropy();
                let protocol_message_channel =
                    super::util::CloneableUnboundedReceiver::from(protocol_message_channel);
                logger.info(format!(
                    "Starting Keygen Protocol with params: i={i}, t={t}, n={n}"
                ));

                let job_id_bytes = additional_params.job_id.to_be_bytes();
                let mix = keccak_256(b"dnfs-cggmp21-keygen");
                let eid_bytes = [&job_id_bytes[..], &mix[..]].concat();
                let eid = dfns_cggmp21::ExecutionId::new(&eid_bytes);
                let mix = keccak_256(b"dnfs-cggmp21-keygen-aux");
                let aux_eid_bytes = [&job_id_bytes[..], &mix[..]].concat();
                let aux_eid = dfns_cggmp21::ExecutionId::new(&aux_eid_bytes);

                let (
                    keygen_tx_to_outbound,
                    keygen_rx_async_proto,
                    _broadcast_tx_to_outbound,
                    _broadcast_rx_from_gadget,
                ) = super::util::create_job_manager_to_async_protocol_channel_split::<_, (), _>(
                    protocol_message_channel.clone(),
                    associated_block_id,
                    associated_retry_id,
                    associated_session_id,
                    associated_task_id,
                    mapping.clone(),
                    id,
                    network.clone(),
                );
                let delivery = (keygen_rx_async_proto, keygen_tx_to_outbound);
                let party = dfns_cggmp21::round_based::MpcParty::connected(delivery);
                let incomplete_key_share = dfns_cggmp21::keygen::<Secp256k1>(eid, i, n)
                    .set_threshold(t)
                    .start(&mut rng, party)
                    .await
                    .map_err(|err| JobError {
                        reason: format!("Keygen protocol error: {err:?}"),
                    })?;

                logger.debug("Finished AsyncProtocol - Incomplete Keygen");

                let (
                    keygen_tx_to_outbound,
                    keygen_rx_async_proto,
                    broadcast_tx_to_outbound,
                    broadcast_rx_from_gadget,
                ) = super::util::create_job_manager_to_async_protocol_channel_split(
                    protocol_message_channel.clone(),
                    associated_block_id,
                    associated_retry_id,
                    associated_session_id,
                    associated_task_id,
                    mapping,
                    id,
                    network,
                );
                let pregenerated_primes = dfns_cggmp21::PregeneratedPrimes::generate(&mut rng);
                let delivery = (keygen_rx_async_proto, keygen_tx_to_outbound);
                let party = dfns_cggmp21::round_based::MpcParty::connected(delivery);
                let aux_info = dfns_cggmp21::aux_info_gen(aux_eid, i, n, pregenerated_primes)
                    .start(&mut rng, party)
                    .await
                    .map_err(|err| JobError {
                        reason: format!("Aux info protocol error: {err:?}"),
                    })?;
                logger.debug("Finished AsyncProtocol - Aux Info");

                let key_share = dfns_cggmp21::KeyShare::make(incomplete_key_share, aux_info)
                    .map_err(|err| JobError {
                        reason: format!("Key share error: {err:?}"),
                    })?;

                logger.debug("Finished AsyncProtocol - Keygen");

                let job_result = handle_public_key_gossip(
                    key_store_for_gossip,
                    &logger,
                    &key_share,
                    t,
                    i,
                    broadcast_tx_to_outbound,
                    broadcast_rx_from_gadget,
                )
                .await?;

                *protocol_output.lock().await = Some((key_share, job_result));
                Ok(())
            })
            .post(async move {
                // TODO: handle protocol blames
                // Store the keys locally, as well as submitting them to the blockchain
                if let Some((local_key, job_result)) = protocol_output_clone.lock().await.take() {
                    key_store
                        .set(additional_params.job_id, local_key)
                        .await
                        .map_err(|err| JobError {
                            reason: format!("Failed to store key: {err:?}"),
                        })?;

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
}

#[allow(clippy::too_many_arguments)]
async fn handle_public_key_gossip<KBE: KeystoreBackend>(
    key_store: ECDSAKeyStore<KBE>,
    logger: &DebugLogger,
    local_key: &KeyShare<Secp256k1>,
    t: u16,
    i: u16,
    broadcast_tx_to_outbound: futures::channel::mpsc::UnboundedSender<PublicKeyGossipMessage>,
    mut broadcast_rx_from_gadget: futures::channel::mpsc::UnboundedReceiver<PublicKeyGossipMessage>,
) -> Result<JobResult, JobError> {
    let serialized_public_key = local_key.shared_public_key().to_bytes(true).to_vec();
    let key_hashed = keccak_256(&serialized_public_key);
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
        .map(|r| r.1)
        .collect::<Vec<_>>();

    let participants = received_participants
        .into_iter()
        .sorted_by_key(|x| x.0)
        .map(|r| r.1 .0.to_vec())
        .collect();

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
        signature_type: DigitalSignatureType::Ecdsa,
        key: serialized_public_key,
        participants,
        signatures,
        threshold: t as _,
    };
    verify_generated_dkg_key_ecdsa(res.clone(), logger);
    Ok(JobResult::DKGPhaseOne(res))
}

fn verify_generated_dkg_key_ecdsa(data: DKGTSSKeySubmissionResult, logger: &DebugLogger) {
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

pub fn verify_signer_from_set_ecdsa(
    maybe_signers: Vec<ecdsa::Public>,
    msg: &[u8],
    signature: &[u8],
) -> (Option<ecdsa::Public>, bool) {
    let mut signer = None;
    let res = maybe_signers.iter().any(|x| {
        if let Some(data) = recover_ecdsa_pub_key(msg, signature) {
            let recovered = &data[..32];
            if x.0[1..].to_vec() == recovered.to_vec() {
                signer = Some(*x);
                true
            } else {
                false
            }
        } else {
            false
        }
    });

    (signer, res)
}

pub fn recover_ecdsa_pub_key(data: &[u8], signature: &[u8]) -> Option<Vec<u8>> {
    const SIGNATURE_LENGTH: usize = 65;
    if signature.len() != SIGNATURE_LENGTH {
        return None;
    }
    let mut sig = [0u8; SIGNATURE_LENGTH];
    sig[..SIGNATURE_LENGTH].copy_from_slice(signature);

    let hash = keccak_256(data);

    sp_io::crypto::secp256k1_ecdsa_recover(&sig, &hash)
        .ok()
        .map(|x| x.to_vec())
}

pub fn to_slice_33(val: &[u8]) -> Option<[u8; 33]> {
    const ECDSA_KEY_LENGTH: usize = 33;
    if val.len() == ECDSA_KEY_LENGTH {
        let mut key = [0u8; ECDSA_KEY_LENGTH];
        key[..ECDSA_KEY_LENGTH].copy_from_slice(val);

        return Some(key);
    }
    None
}
