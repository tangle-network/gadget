use crate::client::{AccountId, ClientWithApi, MpEcdsaClient};
use crate::keystore::KeystoreBackend;
use crate::protocols::state_machine::{CurrentRoundBlame, StateMachineWrapper};
use crate::protocols::util;
use crate::protocols::util::PublicKeyGossipMessage;
use crate::util::DebugLogger;
use crate::MpEcdsaProtocolConfig;
use async_trait::async_trait;
use curv::elliptic::curves::Secp256k1;
use gadget_core::job::{BuiltExecutableJobWrapper, JobBuilder, JobError};
use gadget_core::job_manager::{ProtocolWorkManager, WorkManagerInterface};
use itertools::Itertools;
use multi_party_ecdsa::gg_2020::state_machine::keygen::{Keygen, LocalKey};
use pallet_jobs_rpc_runtime_api::JobsApi;
use parking_lot::Mutex;
use round_based::async_runtime::watcher::StderrWatcher;
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_application_crypto::sp_core::keccak_256;
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::sync::Arc;
use tangle_primitives::jobs::{DKGResult, JobId, JobKey, JobResult, JobType, KeysAndSignatures};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::RwLock;
use webb_gadget::gadget::message::GadgetProtocolMessage;
use webb_gadget::gadget::network::Network;
use webb_gadget::gadget::work_manager::WebbWorkManager;
use webb_gadget::gadget::{Job, WebbGadgetProtocol, WorkManagerConfig};
use webb_gadget::protocol::AsyncProtocol;
use webb_gadget::{Block, BlockImportNotification, FinalityNotification};

pub struct MpEcdsaKeygenProtocol<B: Block, BE, KBE: KeystoreBackend, C, N> {
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
    client: MpEcdsaClient<B, BE, KBE, C>,
    network: N,
    logger: DebugLogger,
) -> Result<MpEcdsaKeygenProtocol<B, BE, KBE, C, N>, Box<dyn Error>>
where
    B: Block,
    BE: Backend<B>,
    C: ClientWithApi<B, BE>,
    KBE: KeystoreBackend,
    N: Network,
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    Ok(MpEcdsaKeygenProtocol {
        client,
        network,
        round_blames: Arc::new(RwLock::new(HashMap::new())),
        logger,
        account_id: config.account_id,
    })
}

#[async_trait]
impl<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>, KBE: KeystoreBackend, N: Network>
    WebbGadgetProtocol<B> for MpEcdsaKeygenProtocol<B, BE, KBE, C, N>
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
            .filter(|r| matches!(r.job_type, JobType::DKG(..)));

        let mut ret = vec![];

        for job in jobs {
            let participants = job
                .job_type
                .get_participants()
                .expect("Should exist for DKG");
            if participants.contains(&self.account_id) {
                let task_id = job.job_id.to_be_bytes();
                let task_id = keccak_256(&task_id);
                let session_id = 0; // We are not interested in sessions for the ECDSA protocol
                let retry_id = job_manager
                    .latest_retry_id(&task_id)
                    .map(|r| r + 1)
                    .unwrap_or(0);

                let additional_params = MpEcdsaKeygenExtraParams {
                    i: participants
                        .iter()
                        .position(|p| p == &self.account_id)
                        .expect("Should exist") as u16,
                    t: job.threshold.expect("T should exist for DKG") as u16,
                    n: participants.len() as u16,
                    job_id: job.job_id,
                    job_key: job.job_type.get_job_key(),
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

struct MpEcdsaKeygenExtraParams {
    i: u16,
    t: u16,
    n: u16,
    job_id: JobId,
    job_key: JobKey,
}

#[async_trait]
impl<B: Block, BE: Backend<B>, KBE: KeystoreBackend, C: ClientWithApi<B, BE>, N: Network>
    AsyncProtocol for MpEcdsaKeygenProtocol<B, BE, KBE, C, N>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    type AdditionalParams = MpEcdsaKeygenExtraParams;
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
        let protocol_output = Arc::new(Mutex::new(None));
        let protocol_output_clone = protocol_output.clone();
        let client = self.client.clone();
        let network = self.network.clone();
        let logger = self.logger.clone();

        let (i, t, n) = (
            additional_params.i,
            additional_params.t,
            additional_params.n,
        );

        Ok(JobBuilder::new()
            .protocol(async move {
                let keygen = Keygen::new(i, t, n).map_err(|err| JobError {
                    reason: format!("Keygen setup error: {err:?}"),
                })?;
                let (current_round_blame_tx, current_round_blame_rx) =
                    tokio::sync::watch::channel(CurrentRoundBlame::default());

                self.round_blames
                    .write()
                    .await
                    .insert(associated_task_id, current_round_blame_rx);

                let (tx_to_outbound, rx_async_proto) =
                    util::create_job_manager_to_async_protocol_channel::<Keygen, _>(
                        protocol_message_rx,
                        associated_block_id,
                        associated_retry_id,
                        associated_session_id,
                        associated_task_id,
                        self.network.clone(),
                    );

                let state_machine_wrapper =
                    StateMachineWrapper::new(keygen, current_round_blame_tx, self.logger.clone());
                let local_key = round_based::AsyncProtocol::new(
                    state_machine_wrapper,
                    rx_async_proto,
                    tx_to_outbound,
                )
                .set_watcher(StderrWatcher)
                .run()
                .await
                .map_err(|err| JobError {
                    reason: format!("Keygen protocol error: {err:?}"),
                })?;

                *protocol_output.lock() = Some(local_key);
                Ok(())
            })
            .post(async move {
                // Check to see if there is any blame at the end of the protocol
                if let Some(blame) = blame.write().await.remove(&associated_task_id) {
                    let blame = blame.borrow();
                    if !blame.blamed_parties.is_empty() {
                        logger.error(format!("Blame: {blame:?}"));
                        return Err(JobError {
                            reason: format!("Keygen blame: {blame:?}"),
                        });
                    }
                }

                // Store the keys locally, as well as submitting them to the blockchain
                if let Some(local_key) = protocol_output_clone.lock().take() {
                    // Serialize via sp encode. Later, in the signing gadget, we will retrieve these serialized
                    // bytes and deserialize them into the LocalKey<Secp256k1> type.
                    // Store for use later in the signing protocol
                    key_store
                        .set(additional_params.job_id, local_key)
                        .await
                        .map_err(|err| JobError {
                            reason: format!("Failed to store key: {err:?}"),
                        })?;

                    // TODO: Proper participant list
                    let participants = vec![];
                    let job_result =
                        handle_public_key_gossip(&logger, &local_key, t, i, participants, network)
                            .await?;

                    client
                        .submit_job_result(
                            additional_params.job_key,
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

async fn handle_public_key_gossip<N: Network>(
    logger: &DebugLogger,
    local_key: &LocalKey<Secp256k1>,
    threshold: u16,
    i: u16,
    participants: Vec<sp_core::ecdsa::Public>,
    network: N,
) -> Result<JobResult, JobError> {
    let serialized_public_key = local_key.public_key().to_bytes(true).to_vec();
    // Sign the public key with our public key
    let signature = vec![];
    let mut received_keys = BTreeMap::new();
    received_keys.insert(i, (serialized_public_key.clone(), signature));

    // TODO: Gossip the public key

    for _ in 0..threshold {
        // TODO: multiplex the network again for keygen
        let message: PublicKeyGossipMessage = receive_message().await?;
        let from = message.from;
        if received_keys.contains_key(&(from as u16)) {
            logger.warn("Received duplicate key");
            continue;
        }

        received_keys.insert(
            from as u16,
            (serialized_public_key.clone(), message.signature),
        );
    }

    // Order and collect the map to ensure symmetric submission to blockchain
    let keys_and_signatures = received_keys.into_iter().sorted().collect();

    Ok(JobResult::DKG(DKGResult {
        key: serialized_public_key,
        participants,
        keys_and_signatures,
        threshold: threshold as _,
    }))
}
