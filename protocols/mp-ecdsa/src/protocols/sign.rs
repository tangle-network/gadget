use crate::client::{AccountId, ClientWithApi, MpEcdsaClient};
use crate::keystore::KeystoreBackend;
use crate::protocols::state_machine::{CurrentRoundBlame, StateMachineWrapper};
use crate::protocols::util;
use crate::protocols::util::VotingMessage;
use crate::util::DebugLogger;
use crate::MpEcdsaProtocolConfig;
use async_trait::async_trait;
use curv::arithmetic::Converter;
use curv::elliptic::curves::Secp256k1;
use curv::BigInt;
use gadget_core::job::{BuiltExecutableJobWrapper, JobBuilder, JobError};
use gadget_core::job_manager::{ProtocolWorkManager, WorkManagerInterface};
use multi_party_ecdsa::gg_2020::party_i::verify;
use multi_party_ecdsa::gg_2020::state_machine::keygen::LocalKey;
use multi_party_ecdsa::gg_2020::state_machine::sign::{
    CompletedOfflineStage, OfflineStage, PartialSignature, SignManual,
};
use pallet_jobs_rpc_runtime_api::JobsApi;
use parity_scale_codec::Encode;
use parking_lot::Mutex;
use round_based::async_runtime::watcher::StderrWatcher;
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_core::ecdsa::Signature;
use sp_core::keccak_256;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tangle_primitives::jobs::{JobId, JobType};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::RwLock;
use webb_gadget::gadget::message::{GadgetProtocolMessage, UserID};
use webb_gadget::gadget::network::Network;
use webb_gadget::gadget::work_manager::WebbWorkManager;
use webb_gadget::gadget::{Job, WebbGadgetProtocol, WorkManagerConfig};
use webb_gadget::protocol::AsyncProtocol;
use webb_gadget::{Block, BlockImportNotification, FinalityNotification};

pub struct MpEcdsaSigningProtocol<B: Block, BE, KBE: KeystoreBackend, C, N> {
    client: MpEcdsaClient<B, BE, KBE, C>,
    network: N,
    round_blames: Arc<
        RwLock<
            HashMap<
                <WebbWorkManager as WorkManagerInterface>::TaskID,
                tokio::sync::watch::Receiver<CurrentRoundBlame>,
            >,
        >,
    >,
    logger: DebugLogger,
    account_id: AccountId,
}

pub async fn create_protocol<B, BE, KBE, C, N>(
    config: &MpEcdsaProtocolConfig,
    logger: DebugLogger,
    client: MpEcdsaClient<B, BE, KBE, C>,
    network: N,
) -> Result<MpEcdsaSigningProtocol<B, BE, KBE, C, N>, Box<dyn Error>>
where
    B: Block,
    BE: Backend<B>,
    C: ClientWithApi<B, BE>,
    KBE: KeystoreBackend,
    N: Network,
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    Ok(MpEcdsaSigningProtocol {
        client,
        network,
        round_blames: Arc::new(Default::default()),
        logger,
        account_id: config.account_id,
    })
}

#[async_trait]
impl<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>, KBE: KeystoreBackend, N: Network>
    WebbGadgetProtocol<B> for MpEcdsaSigningProtocol<B, BE, KBE, C, N>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    async fn get_next_jobs(
        &self,
        notification: &FinalityNotification<B>,
        now: u64,
        job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) -> Result<Option<Vec<Job>>, webb_gadget::Error> {
        let jobs = self
            .client
            .query_jobs_by_validator(notification.hash, self.account_id)
            .await?
            .into_iter()
            .filter(|r| matches!(r.job_type, JobType::DKGSignature(..)));

        let mut ret = vec![];

        for job in jobs {
            let participants = job.participants.expect("Should exist for stage 2 signing");
            if participants.contains(&self.account_id) {
                let task_id = job.job_id.to_be_bytes();
                let task_id = keccak_256(&task_id);
                let session_id = 0; // We are not interested in sessions for the ECDSA protocol
                                    // For the retry ID, get the latest retry and increment by 1 to set the next retry ID
                                    // If no retry ID exists, it means the job is new, thus, set the retry_id to 0
                let retry_id = job_manager
                    .latest_retry_id(&task_id)
                    .map(|r| r + 1)
                    .unwrap_or(0);

                if let Some(key) = self
                    .client
                    .key_store
                    .get(&job.job_id)
                    .await
                    .map_err(|err| webb_gadget::Error::ClientError {
                        err: err.to_string(),
                    })?
                {
                    let additional_params = MpEcdsaSigningExtraParams {
                        i: participants
                            .iter()
                            .position(|p| p == &self.account_id)
                            .expect("Should exist") as u16,
                        t: job.threshold.expect("T should exist for stage 2 signing") as u16,
                        signers: (0..participants.len())
                            .into_iter()
                            .map(|r| r as u16)
                            .collect(),
                        job_id: job.job_id,
                        key,
                    };

                    let job = self
                        .create(session_id, now, retry_id, task_id, additional_params)
                        .await?;

                    ret.push(job);
                } else {
                    self.logger.warn(format!(
                        "No key found for job ID: {job_id:?}",
                        job_id = job.job_id
                    ));
                }
            }
        }

        Ok(Some(ret))
    }

    async fn process_block_import_notification(
        &self,
        _notification: BlockImportNotification<B>,
        _job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) -> Result<(), webb_gadget::Error> {
        Ok(())
    }

    async fn process_error(
        &self,
        error: webb_gadget::Error,
        _job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) {
        log::error!(target: "mp-ecdsa", "Error: {error:?}");
    }

    fn get_work_manager_config(&self) -> WorkManagerConfig {
        WorkManagerConfig {
            interval: None, // Manual polling
            max_active_tasks: 8,
            max_pending_tasks: 8,
        }
    }
}

struct MpEcdsaSigningExtraParams {
    i: u16,
    t: u16,
    signers: Vec<u16>,
    job_id: JobId,
    key: LocalKey<Secp256k1>,
}

#[async_trait]
impl<B: Block, BE: Backend<B>, KBE: KeystoreBackend, C: ClientWithApi<B, BE>, N: Network>
    AsyncProtocol for MpEcdsaSigningProtocol<B, BE, KBE, C, N>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    type AdditionalParams = MpEcdsaSigningExtraParams;
    async fn generate_protocol_from(
        &self,
        associated_block_id: <WebbWorkManager as WorkManagerInterface>::Clock,
        associated_retry_id: <WebbWorkManager as WorkManagerInterface>::RetryID,
        associated_session_id: <WebbWorkManager as WorkManagerInterface>::SessionID,
        associated_task_id: <WebbWorkManager as WorkManagerInterface>::TaskID,
        protocol_message_rx: UnboundedReceiver<GadgetProtocolMessage>,
        additional_params: Self::AdditionalParams,
    ) -> Result<BuiltExecutableJobWrapper, JobError> {
        let blame = self.round_blames.clone();
        let debug_logger_post = self.logger.clone();
        let debug_logger_proto = debug_logger_post.clone();
        let protocol_output = Arc::new(Mutex::new(None));
        let protocol_output_clone = protocol_output.clone();
        let client = self.client.clone();

        Ok(JobBuilder::new()
            .protocol(async move {
                let (i, signers, t, key) = (
                    additional_params.i,
                    additional_params.signers,
                    additional_params.t,
                    additional_params.key,
                );

                let offline_i = get_offline_i(i, &signers);

                let signing =
                    OfflineStage::new(offline_i, signers, key).map_err(|err| JobError {
                        reason: format!("Failed to create offline stage: {err:?}"),
                    })?;
                let (current_round_blame_tx, current_round_blame_rx) =
                    tokio::sync::watch::channel(CurrentRoundBlame::default());

                self.round_blames
                    .write()
                    .await
                    .insert(associated_task_id, current_round_blame_rx);

                let (
                    tx_to_outbound_offline,
                    rx_async_proto_offline,
                    tx_to_outbound_voting,
                    rx_async_proto_voting,
                ) = util::create_job_manager_to_async_protocol_channel_signing(
                    protocol_message_rx,
                    associated_block_id,
                    associated_retry_id,
                    associated_session_id,
                    associated_task_id,
                    self.network.clone(),
                );

                let state_machine_wrapper =
                    StateMachineWrapper::new(signing, current_round_blame_tx, self.logger.clone());
                let completed_offline_stage = round_based::AsyncProtocol::new(
                    state_machine_wrapper,
                    rx_async_proto_offline,
                    tx_to_outbound_offline,
                )
                .set_watcher(StderrWatcher)
                .run()
                .await
                .map_err(|err| JobError {
                    reason: format!("Keygen protocol error: {err:?}"),
                })?;

                debug_logger_proto.info(format!(
                    "*** Completed offline stage: {:?}",
                    completed_offline_stage.public_key()
                ));

                // We will sign over the unique task ID
                let message = BigInt::from_bytes(&associated_task_id);

                // Conclude with the voting stage
                let signature = voting_stage(
                    offline_i,
                    t,
                    message,
                    completed_offline_stage,
                    rx_async_proto_voting,
                    tx_to_outbound_voting,
                    &debug_logger_proto,
                )
                .await?;
                *protocol_output.lock() = Some(signature);
                Ok(())
            })
            .post(async move {
                // Check to see if there is any blame at the end of the protocol
                if let Some(blame) = blame.write().await.remove(&associated_task_id) {
                    let blame = blame.borrow();
                    if !blame.blamed_parties.is_empty() {
                        debug_logger_post.error(format!("Blame: {blame:?}"));
                        return Err(JobError {
                            reason: format!("Signing blame: {blame:?}"),
                        });
                    }
                }

                // Submit the protocol output to the blockchain
                if let Some(signature) = protocol_output_clone.lock().take() {
                    let serialized = signature.encode();
                    client
                        .submit_job_result(additional_params.job_id, serialized)
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

/// Finds our index inside the list of signers, then adds 1 since the Offline state machine
/// expects the index to start from 1.
fn get_offline_i(i: u16, signers: &[u16]) -> u16 {
    (signers.iter().position(|s| s == &i).expect("Should exist") + 1) as u16
}

async fn voting_stage(
    offline_i: u16,
    threshold: u16,
    message: BigInt,
    completed_offline_stage: CompletedOfflineStage,
    mut msg_rx: UnboundedReceiver<VotingMessage>,
    msg_tx: UnboundedSender<VotingMessage>,
    debug_logger: &DebugLogger,
) -> Result<Signature, JobError> {
    let offline_stage_pub_key = completed_offline_stage.public_key().clone();
    let (signing, partial_signature) = SignManual::new(message.clone(), completed_offline_stage)
        .map_err(|err| JobError {
            reason: format!("Failed to create voting stage: {err:?}"),
        })?;

    let partial_sig_bytes = bincode2::serialize(&partial_signature).map_err(|err| JobError {
        reason: format!("Failed to serialize partial signature: {err:?}"),
    })?;

    let payload = VotingMessage {
        from: offline_i as UserID,
        to: None, // Broadcast to everyone
        payload: partial_sig_bytes,
    };

    msg_tx.send(payload).map_err(|err| JobError {
        reason: format!("Failed to send partial signature: {err:?}"),
    })?;

    let mut sigs = HashMap::with_capacity(threshold as _);

    while let Some(vote_message) = msg_rx.recv().await {
        let vote_message: VotingMessage = vote_message;
        if sigs.contains_key(&vote_message.from) {
            debug_logger.warn(format!(
                "Received duplicate signature from {}",
                vote_message.from
            ));
            continue;
        }

        if let Ok(p_sig) = bincode2::deserialize::<PartialSignature>(&vote_message.payload) {
            sigs.insert(vote_message.from, p_sig);

            if sigs.len() == threshold as usize {
                break;
            }
        } else {
            debug_logger.warn(format!(
                "Received invalid signature bytes from {}",
                vote_message.from
            ));
        }
    }

    if sigs.len() != threshold as usize {
        return Err(JobError {
            reason: format!(
                "Failed to collect enough signatures: {}/{}",
                sigs.len(),
                threshold
            ),
        });
    }

    // Aggregate and complete the signature
    let sigs: Vec<PartialSignature> = sigs.into_values().collect();
    let signature = signing.complete(&sigs).map_err(|err| JobError {
        reason: format!("Failed to complete signature: {err:?}"),
    })?;

    // Verify the signature
    verify(&signature, &offline_stage_pub_key, &message).map_err(|err| JobError {
        reason: format!("Failed to verify signature: {err:?}"),
    })?;

    // Convert the signature to a substrate-compatible format
    crate::util::convert_signature(&signature).ok_or_else(|| JobError {
        reason: "Failed to convert signature to Substrate-compatible format".to_string(),
    })
}
