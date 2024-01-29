use crate::keygen::payloads::RoundPayload;
use crate::protocol::state_machine::payloads::RoundPayload;
use async_trait::async_trait;
use gadget_common::client::{AccountId, ClientWithApi, JobsClient, PalletSubmitter};
use gadget_common::config::{DebugLogger, GadgetProtocol, JobsApi, Network, ProvideRuntimeApi};
use gadget_common::gadget::message::{GadgetProtocolMessage, UserID};
use gadget_common::gadget::work_manager::WorkManager;
use gadget_common::keystore::{GenericKeyStore, KeystoreBackend};
use gadget_common::sp_core::{keccak_256, Pair};
use gadget_common::{Backend, Block, JobError, WorkManagerInterface};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tangle_primitives::jobs::{DKGTSSKeySubmissionResult, DigitalSignatureType, JobId, JobResult};
use tangle_primitives::roles::RoleType;

pub struct BlsKeygenProtocol<
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
    pub pallet_tx: Arc<dyn PalletSubmitter>,
    pub keystore: GenericKeyStore<KBE, gadget_common::sp_core::ecdsa::Pair>,
}

#[async_trait]
impl<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>, N: Network, KBE: KeystoreBackend>
    GadgetProtocol<B, BE, C> for BlsKeygenProtocol<B, BE, C, N, KBE>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    async fn create_next_job(
        &self,
        job: JobInitMetadata,
    ) -> Result<<Self as AsyncProtocol>::AdditionalParams, Error> {
        let job_id = job.job_id;
        let task_hash = job.task_id;
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
                .expect("Should exist")
                + 1) as u16,
            t: threshold,
            n: participants.len() as u16,
            role_type,
            task_hash,
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
    task_hash: [u8; 32],
    user_id_to_account_id_mapping: Arc<HashMap<UserID, AccountId>>,
}

#[async_trait]
impl<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>, N: Network, KBE: KeystoreBackend>
    AsyncProtocol for BlsKeygenProtocol<B, BE, C, N, KBE>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    type AdditionalParams = BlsKeygenAdditionalParams;

    async fn generate_protocol_from(
        &self,
        associated_block_id: <WorkManager as WorkManagerInterface>::Clock,
        associated_retry_id: <WorkManager as WorkManagerInterface>::RetryID,
        associated_session_id: <WorkManager as WorkManagerInterface>::SessionID,
        associated_task_id: <WorkManager as WorkManagerInterface>::TaskID,
        mut protocol_message_rx: tokio::sync::mpsc::UnboundedReceiver<GadgetProtocolMessage>,
        additional_params: Self::AdditionalParams,
    ) -> Result<BuiltExecutableJobWrapper, JobError> {
        let threshold = NonZeroUsize::new(additional_params.t as usize).expect(" T should be > 0");
        let n = NonZeroUsize::new(additional_params.n as usize).expect("N should be > 0");
        let i = NonZeroUsize::new(additional_params.i as usize).expect("I should be > 0");
        let network = self.network.clone();
        let result = Arc::new(tokio::sync::Mutex::new(None));
        let result_clone = result.clone();
        let keystore = self.keystore.clone();
        let job_id = additional_params.job_id;
        let pallet_tx = self.pallet_tx.clone();
        let role_type = additional_params.role_type;

        Ok(JobBuilder::new()
            .protocol(async move {
                let params = Parameters::<Group>::new(threshold, n);
                let mut me = SecretParticipant::<Group>::new(i, params).unwrap();

                // round 1: Everyone generated broadcast payload + p2p payload, then, sends to everyone
                let count_required = additional_params.n - 1;
                let (broadcast, p2p) = me.round1().map_err(|e| JobError {
                    reason: e.to_string(),
                })?;

                // Broadcast the broadcast payload
                send_message(
                    associated_block_id,
                    associated_retry_id,
                    associated_session_id,
                    associated_task_id,
                    &network,
                    RoundPayload::Round1(payloads::Round1Payload::Broadcast(broadcast)),
                    self.account_id.clone(),
                    additional_params.i as UserID,
                    None,
                    &additional_params.user_id_to_account_id_mapping,
                )
                .await?;

                // For each p2p payload, send to the peer
                for (to, p2p) in p2p {
                    send_message(
                        associated_block_id,
                        associated_retry_id,
                        associated_session_id,
                        associated_task_id,
                        &network,
                        RoundPayload::Round1(payloads::Round1Payload::P2P(p2p)),
                        self.account_id.clone(),
                        additional_params.i as UserID,
                        Some(to as UserID),
                        &additional_params.user_id_to_account_id_mapping,
                    )
                    .await?;
                }

                // Now, receive all the broadcast and p2p messages in seperate BTreeMaps
                let mut broadcast_map = BTreeMap::new();
                let mut p2p_map = BTreeMap::new();
                let mut count = 0;
                while count < count_required {
                    let message = protocol_message_rx.recv().await.ok_or(JobError {
                        reason: "Channel closed".to_string(),
                    })?;
                    let payload: RoundPayload =
                        bincode2::deserialize(&message.payload).map_err(|e| JobError {
                            reason: e.to_string(),
                        })?;
                    match payload {
                        RoundPayload::Round1(payload) => match payload {
                            payloads::Round1Payload::Broadcast(broadcast) => {
                                broadcast_map.insert(message.from as usize, broadcast);
                            }
                            payloads::Round1Payload::P2P(p2p) => {
                                p2p_map.insert(message.from as usize, p2p);
                            }
                        },
                        _ => {
                            return Err(JobError {
                                reason: "Unexpected payload".to_string(),
                            })
                        }
                    }
                    count += 1;
                }

                let round2_broadcast =
                    me.round2(broadcast_map, p2p_map).map_err(|err| JobError {
                        reason: err.to_string(),
                    })?;

                // Broadcast the round2 data
                send_message(
                    associated_block_id,
                    associated_retry_id,
                    associated_session_id,
                    associated_task_id,
                    &network,
                    RoundPayload::Round2(round2_broadcast),
                    self.account_id.clone(),
                    additional_params.i as UserID,
                    None,
                    &additional_params.user_id_to_account_id_mapping,
                )
                .await?;

                // Receive the broadcasts inside a BtreeMap
                let mut round2_broadcast_map = BTreeMap::new();
                let mut count = 0;
                while count < count_required {
                    let message = protocol_message_rx.recv().await.ok_or(JobError {
                        reason: "Channel closed".to_string(),
                    })?;
                    let payload: RoundPayload =
                        bincode2::deserialize(&message.payload).map_err(|e| JobError {
                            reason: e.to_string(),
                        })?;
                    match payload {
                        RoundPayload::Round2(payload) => {
                            round2_broadcast_map.insert(message.from as usize, payload);
                        }
                        _ => {
                            return Err(JobError {
                                reason: "Unexpected payload".to_string(),
                            })
                        }
                    }
                    count += 1;
                }

                let round3_broadcast =
                    me.round3(&round2_broadcast_map).map_err(|err| JobError {
                        reason: err.to_string(),
                    })?;

                // Broadcast the round3 data
                send_message(
                    associated_block_id,
                    associated_retry_id,
                    associated_session_id,
                    associated_task_id,
                    &network,
                    RoundPayload::Round3(round3_broadcast),
                    self.account_id.clone(),
                    additional_params.i as UserID,
                    None,
                    &additional_params.user_id_to_account_id_mapping,
                )
                .await?;

                // Receive the broadcasts inside a BtreeMap
                let mut round3_broadcast_map = BTreeMap::new();
                let mut count = 0;
                while count < count_required {
                    let message = protocol_message_rx.recv().await.ok_or(JobError {
                        reason: "Channel closed".to_string(),
                    })?;
                    let payload: RoundPayload =
                        bincode2::deserialize(&message.payload).map_err(|e| JobError {
                            reason: e.to_string(),
                        })?;
                    match payload {
                        RoundPayload::Round3(payload) => {
                            round3_broadcast_map.insert(message.from as usize, payload);
                        }
                        _ => {
                            return Err(JobError {
                                reason: "Unexpected payload".to_string(),
                            })
                        }
                    }
                    count += 1;
                }

                let round4_broadcast =
                    me.round4(&round3_broadcast_map).map_err(|err| JobError {
                        reason: err.to_string(),
                    })?;

                // Broadcast the round4 data
                send_message(
                    associated_block_id,
                    associated_retry_id,
                    associated_session_id,
                    associated_task_id,
                    &network,
                    RoundPayload::Round4(round4_broadcast),
                    self.account_id.clone(),
                    additional_params.i as UserID,
                    None,
                    &additional_params.user_id_to_account_id_mapping,
                )
                .await?;

                // Receive the broadcasts inside a BtreeMap
                let mut round4_broadcast_map = BTreeMap::new();
                let mut count = 0;
                while count < count_required {
                    let message = protocol_message_rx.recv().await.ok_or(JobError {
                        reason: "Channel closed".to_string(),
                    })?;
                    let payload =
                        bincode2::deserialize(&message.payload).map_err(|e| JobError {
                            reason: e.to_string(),
                        })?;
                    match payload {
                        RoundPayload::Round4(payload) => {
                            round4_broadcast_map.insert(message.from as usize, payload);
                        }
                        _ => {
                            return Err(JobError {
                                reason: "Unexpected payload".to_string(),
                            })
                        }
                    }
                    count += 1;
                }

                me.round5(&round4_broadcast_map).map_err(|err| JobError {
                    reason: err.to_string(),
                })?;

                let public_key = me.get_public_key().ok_or_else(|| JobError {
                    reason: "Failed to get public key".to_string(),
                })?;

                let secret_share = me.get_secret_share().ok_or_else(|| JobError {
                    reason: "Failed to get secret share".to_string(),
                })?;

                let secret =
                    <Vec<u8> as Share>::from_field_element(me.get_id() as u8, secret_share)
                        .map_err(|err| JobError {
                            reason: err.to_string(),
                        })?;

                let job_result = handle_public_key_broadcast(
                    &keystore,
                    public_key,
                    additional_params.i,
                    additional_params.t,
                    &mut protocol_message_rx,
                    associated_block_id,
                    associated_retry_id,
                    associated_session_id,
                    associated_task_id,
                    &network,
                    additional_params.i as UserID,
                    &additional_params.user_id_to_account_id_mapping,
                )
                .await?;

                *result.lock().await = Some((job_result, secret));

                Ok(())
            })
            .post(async move {
                if let Some((job_result, secret)) = result_clone.lock().await {
                    keystore.set(job_id, secret).await.map_err(|err| JobError {
                        reason: err.to_string(),
                    })?;

                    pallet_tx
                        .submit_job_result(role_type, job_id, job_result)
                        .await
                        .map_err(|err| JobError {
                            reason: err.to_string(),
                        })?;
                }
            })
            .build())
    }
}

async fn handle_public_key_broadcast<KBE: KeystoreBackend, N: Network>(
    key_store: &GenericKeyStore<KBE, gadget_common::sp_core::ecdsa::Pair>,
    public_key: Vec<u8>,
    i: u16,
    t: u16,
    protocol_message_rx: &mut tokio::sync::mpsc::UnboundedReceiver<GadgetProtocolMessage>,
    associated_block_id: <WorkManager as WorkManagerInterface>::Clock,
    associated_retry_id: <WorkManager as WorkManagerInterface>::RetryID,
    associated_session_id: <WorkManager as WorkManagerInterface>::SessionID,
    task_hash: <WorkManager as WorkManagerInterface>::TaskID,
    network: &N,
    my_idx: UserID,
    user_id_to_account_id_mapping: &Arc<HashMap<UserID, AccountId>>,
) -> Result<JobResult, JobError> {
    let key_hashed = keccak_256(&public_key);
    let signature = key_store.pair().sign_prehashed(&key_hashed).0.to_vec();
    let my_id = key_store.pair().public();

    let mut received_signatures = BTreeMap::new();
    received_signatures.insert(i, signature.clone());

    let broadcast_message = RoundPayload::PublicKeyGossip(signature);
    send_message(
        associated_block_id,
        associated_retry_id,
        associated_session_id,
        task_hash,
        network,
        broadcast_message,
        my_id,
        my_idx,
        None,
        user_id_to_account_id_mapping,
    )
    .await?;

    // Receive t signatures
    let mut count = 0;
    while count < t {
        let message = protocol_message_rx.recv().await.ok_or(JobError {
            reason: "Channel closed".to_string(),
        })?;
        let payload: RoundPayload =
            bincode2::deserialize(&message.payload).map_err(|e| JobError {
                reason: e.to_string(),
            })?;
        match payload {
            RoundPayload::PublicKeyGossip(signature) => {
                received_signatures.insert(message.from as u16, signature);
            }
            _ => {
                return Err(JobError {
                    reason: "Unexpected payload".to_string(),
                })
            }
        }
        count += 1;
    }

    let participants = user_id_to_account_id_mapping
        .into_iter()
        .sorted_by_key(|x| x.0)
        .map(|r| r.1 .0.to_vec())
        .collect();

    let signatures = received_signatures
        .into_iter()
        .sorted_by_key(|x| x.0)
        .map(|r| r.1)
        .collect();

    Ok(JobResult::DKGPhaseOne(DKGTSSKeySubmissionResult {
        signature_type: DigitalSignatureType::Bls381,
        key: public_key,
        participants,
        signatures,
        threshold: t as u8,
    }))
}

async fn send_message<N: Network>(
    associated_block_id: <WorkManager as WorkManagerInterface>::Clock,
    associated_retry_id: <WorkManager as WorkManagerInterface>::RetryID,
    associated_session_id: <WorkManager as WorkManagerInterface>::SessionID,
    task_hash: <WorkManager as WorkManagerInterface>::TaskID,
    network: &N,
    round_payload: RoundPayload,
    my_id: AccountId,
    my_idx: UserID,
    to: Option<UserID>,
    user_id_to_account_id_mapping: &Arc<HashMap<UserID, AccountId>>,
) -> Result<(), JobError> {
    let (to, to_network_id) = if let Some(to_val) = to {
        let to = user_id_to_account_id_mapping
            .get(&to_val)
            .copied()
            .expect("Should exist");
        (Some(to_val), Some(to))
    } else {
        (None, None)
    };

    let payload = bincode2::serialize(&round_payload).expect("Should serialize");
    let message = GadgetProtocolMessage {
        associated_block_id,
        associated_retry_id,
        associated_session_id,
        task_hash,
        from: my_idx,
        to,
        payload,
        from_network_id: Some(my_id),
        to_network_id,
    };

    network.send_message(message).await?;
    Ok(())
}
