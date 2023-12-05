use std::collections::HashMap;
use crate::MpEcdsaProtocolConfig;
use async_trait::async_trait;
use gadget_core::job_manager::{ProtocolWorkManager, WorkManagerInterface};
use std::error::Error;
use std::sync::Arc;
use curv::arithmetic::Converter;
use curv::BigInt;
use curv::elliptic::curves::Secp256k1;
use multi_party_ecdsa::gg_2020::state_machine::keygen::{Keygen, LocalKey};
use multi_party_ecdsa::gg_2020::state_machine::sign::{CompletedOfflineStage, OfflineStage, SignManual};
use round_based::async_runtime::watcher::StderrWatcher;
use sc_client_api::Backend;
use sp_core::keccak_256;
use tangle_primitives::jobs::{JobId, JobType};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::RwLock;
use gadget_core::job::{BuiltExecutableJobWrapper, JobBuilder, JobError};
use webb_gadget::gadget::work_manager::WebbWorkManager;
use webb_gadget::gadget::{Job, WebbGadgetProtocol, WorkManagerConfig};
use webb_gadget::{Block, BlockImportNotification, FinalityNotification};
use webb_gadget::gadget::message::GadgetProtocolMessage;
use webb_gadget::protocol::AsyncProtocol;
use crate::client::{AccountId, ClientWithApi, MpEcdsaClient};
use crate::keystore::KeystoreBackend;
use crate::network::GossipNetwork;
use crate::protocols::keygen::MpEcdsaKeygenProtocol;
use crate::protocols::state_machine::{CurrentRoundBlame, StateMachineWrapper};
use crate::protocols::util;
use crate::util::DebugLogger;

pub struct MpEcdsaSigningProtocol<B, BE, KBE, C> {
    client: MpEcdsaClient<B, BE, KBE, C>,
    network: GossipNetwork,
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

pub async fn create_protocol(
    _config: &MpEcdsaProtocolConfig,
) -> Result<MpEcdsaSigningProtocol, Box<dyn Error>> {
    Ok(MpEcdsaSigningProtocol {})
}

#[async_trait]
impl<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>, KBE: KeystoreBackend> WebbGadgetProtocol<B>
for MpEcdsaSigningProtocol<B, BE, KBE, C>
{
    async fn get_next_jobs(
        &self,
        _notification: &FinalityNotification<B>,
        now: u64,
        _job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) -> Result<Option<Vec<Job>>, webb_gadget::Error> {
        let jobs = self
            .client
            .query_jobs_by_validator(self.account_id)
            .await?
            .into_iter()
            .filter(|r| matches!(r.job_type, JobType::DKGSignature(..)));

        let mut ret = vec![];

        for job in jobs {
            let participants = job
                .participants
                .expect("Should exist for DKG");
            if participants.contains(&self.account_id) {
                let task_id = job.job_id.to_be_bytes();
                let task_id = keccak_256(&task_id);
                let session_id = 0; // We are not interested in sessions for the ECDSA protocol
                let retry_id = 0; // TODO: query the job manager for the retry id

                let additional_params = MpEcdsaKeygenExtraParams {
                    i: participants
                        .iter()
                        .position(|p| p == &self.account_id)
                        .expect("Should exist") as u16,
                    t: job.threshold.expect("T should exist for DKG") as u16,
                    n: participants.len() as u16,
                    job_id: job.job_id,
                };

                let job = self
                    .create(session_id, now, retry_id, task_id, additional_params)
                    .await?;

                ret.push(job);
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
            max_active_tasks: 1,
            max_pending_tasks: 1,
        }
    }
}

struct MpEcdsaSigningExtraParams {
    i: u16,
    signers: Vec<u16>,
    job_id: JobId,
    key: LocalKey<Secp256k1>
}

#[async_trait]
impl<B, BE, KBE: KeystoreBackend, C> AsyncProtocol for MpEcdsaSigningProtocol<B, BE, KBE, C> {
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
        let key_store = self.client.key_store.clone();
        let blame = self.round_blames.clone();
        let debug_logger_post = self.logger.clone();
        let debug_logger_proto = debug_logger_post.clone();

        Ok(JobBuilder::new()
            .post(async move {
                // Check to see if there is any blame at the end of the protocol
                if let Some(blame) = blame.write().await.remove(&associated_task_id) {
                    let blame = blame.borrow();
                    if !blame.blamed_parties.is_empty() {
                        debug_logger_post.error(format!("Blame: {blame:?}"));
                        return Err(JobError {
                            reason: format!("Blame: {blame:?}"),
                        });
                    }
                }

                Ok(())
            })
            .build(async move {
                let (i, signers, key) = (
                    additional_params.i,
                    additional_params.signers,
                    additional_params.key,
                );

                let offline_i = get_offline_i(i, &signers);

                let signing = OfflineStage::new(offline_i, signers, key)?;
                let (current_round_blame_tx, current_round_blame_rx) =
                    tokio::sync::watch::channel(CurrentRoundBlame::default());

                self.round_blames
                    .write()
                    .await
                    .insert(associated_task_id, current_round_blame_rx);

                let (tx_to_outbound, rx_async_proto) =
                    util::create_job_manager_to_async_protocol_channel::<OfflineStage, _>(
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
                    rx_async_proto,
                    tx_to_outbound,
                )
                    .set_watcher(StderrWatcher)
                    .run()
                    .await?;

                debug_logger_proto.info(format!(
                    "*** Completed offline stage: {completed_offline_stage:?}"
                ));

                // We will sign over the unique task ID
                let message = BigInt::from_bytes(&associated_task_id);

                voting_stage(offline_i, message, completed_offline_stage).await
            }))
    }
}

/// Finds our index inside the list of signers, then adds 1 since the Offline state machine
/// expects the index to start from 1.
fn get_offline_i(i: u16, signers: &[u16]) -> u16 {
    (signers.iter().position(|s| s == &i).expect("Should exist") + 1) as u16
}

async fn voting_stage(offline_i: u16, message: BigInt, completed_offline_stage: CompletedOfflineStage) -> Result<(), JobError> {
    let (signing, partial_signature) = SignManual::new(message, completed_offline_stage)
        .map_err(|err| JobError { reason: format!("Failed to create voting stage: {err:?}") })?;

    let partial_sig_bytes = bincode2::serialize(&partial_signature)
        .map_err(|err| JobError { reason: format!("Failed to serialize partial signature: {err:?}") })?;

    // TODO: multiplex the stream for signing and voting messages
    let payload = DKGVoteMessage {
        party_ind: *offline_i.as_ref(),
        // use the hash of proposal as "round key" ONLY for purposes of ensuring
        // uniqueness We only want voting to happen amongst voters under the SAME
        // proposal, not different proposals This is now especially necessary since we
        // are allowing for parallelism now
        round_key: Vec::from(&hash_of_proposal as &[u8]),
        partial_signature: partial_sig_bytes,
        unsigned_proposal_hash,
    };
}

enum SigningMessage {
    Signing { },
    Voting(VotingMessage)
}

struct VotingMessage {
    party_ind: u16,
    round_key: Vec<u8>,
    partial_signature: Vec<u8>,
    unsigned_proposal_hash: Vec<u8>,
}