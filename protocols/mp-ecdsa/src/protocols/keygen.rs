use crate::mock::id_to_public;
use crate::protocols::state_machine::{CurrentRoundBlame, StateMachineWrapper};
use crate::protocols::util;
use crate::protocols::util::PublicKeyGossipMessage;
use crate::MpEcdsaProtocolConfig;
use async_trait::async_trait;
use curv::elliptic::curves::Secp256k1;
use frame_support::Hashable;
use gadget_common::client::{AccountId, ClientWithApi, MpEcdsaClient};
use gadget_common::debug_logger::DebugLogger;
use gadget_common::gadget::message::{GadgetProtocolMessage, UserID};
use gadget_common::gadget::network::Network;
use gadget_common::gadget::work_manager::WebbWorkManager;
use gadget_common::gadget::{Job, WebbGadgetProtocol, WorkManagerConfig};
use gadget_common::keystore::KeystoreBackend;
use gadget_common::protocol::AsyncProtocol;
use gadget_common::{Block, BlockImportNotification, FinalityNotification};
use gadget_core::job::{BuiltExecutableJobWrapper, JobBuilder, JobError};
use gadget_core::job_manager::{ProtocolWorkManager, WorkManagerInterface};
use itertools::Itertools;
use multi_party_ecdsa::gg_2020::state_machine::keygen::{Keygen, LocalKey};
use pallet_jobs_rpc_runtime_api::JobsApi;
use round_based::async_runtime::watcher::StderrWatcher;
use round_based::{Msg, StateMachine};
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_application_crypto::sp_core::keccak_256;
use sp_application_crypto::RuntimePublic;
use sp_core::crypto::KeyTypeId;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tangle_primitives::jobs::{
    DKGResult, DKGTSSPhaseOneJobType, DkgKeyType, JobId, JobKey, JobResult, JobType,
    RpcResponseJobsData,
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::RwLock;

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
) -> MpEcdsaKeygenProtocol<B, BE, KBE, C, N>
where
    B: Block,
    BE: Backend<B>,
    C: ClientWithApi<B, BE>,
    KBE: KeystoreBackend,
    N: Network,
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    MpEcdsaKeygenProtocol {
        client,
        network,
        round_blames: Arc::new(RwLock::new(HashMap::new())),
        logger,
        account_id: config.account_id,
    }
}

#[async_trait]
impl<
        B: Block,
        BE: Backend<B> + 'static,
        C: ClientWithApi<B, BE>,
        KBE: KeystoreBackend,
        N: Network,
    > WebbGadgetProtocol<B> for MpEcdsaKeygenProtocol<B, BE, KBE, C, N>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    async fn get_next_jobs(
        &self,
        notification: &FinalityNotification<B>,
        now: u64,
        job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) -> Result<Option<Vec<Job>>, gadget_common::Error> {
        self.logger.info(format!("At finality notification {now}"));
        /*let jobs = self
        .client
        .query_jobs_by_validator(notification.hash, self.account_id)
        .await?
        .into_iter()
        .filter(|r| matches!(r.job_type, JobType::DKGTSSPhaseOne(..)));*/

        const N: usize = 3;
        const T: usize = N - 1;
        let jobs = if now == 3 {
            let identities = (0..N).map(|i| id_to_public(i as u8)).collect::<Vec<_>>();
            vec![RpcResponseJobsData {
                job_id: 0,
                job_type: JobType::DKGTSSPhaseOne(DKGTSSPhaseOneJobType {
                    participants: identities.clone(),
                    threshold: T as u8,
                    permitted_caller: None,
                    key_type: DkgKeyType::Ecdsa,
                }),
                participants: Some(identities.clone()),
                threshold: Some(T as u8),
                key: None,
            }]
        } else {
            vec![]
        };

        let mut ret = vec![];

        for job in jobs {
            let participants = job
                .job_type
                .clone()
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

                let user_id_to_account_id_mapping = Arc::new(
                    participants
                        .clone()
                        .into_iter()
                        .enumerate()
                        .map(|r| ((r.0 + 1) as UserID, r.1))
                        .collect(),
                );

                let additional_params = MpEcdsaKeygenExtraParams {
                    i: participants
                        .iter()
                        .position(|p| p == &self.account_id)
                        .expect("Should exist") as u16,
                    t: job.threshold.expect("T should exist for DKG") as u16,
                    n: participants.len() as u16,
                    job_id: job.job_id,
                    job_key: job.job_type.get_job_key(),
                    user_id_to_account_id_mapping,
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
    ) -> Result<(), gadget_common::Error> {
        Ok(())
    }

    async fn process_error(
        &self,
        error: gadget_common::Error,
        _job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) {
        log::error!(target: "mp-ecdsa", "Error: {error:?}");
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

pub struct MpEcdsaKeygenExtraParams {
    i: u16,
    t: u16,
    n: u16,
    job_id: JobId,
    job_key: JobKey,
    user_id_to_account_id_mapping: Arc<HashMap<UserID, AccountId>>,
}

#[async_trait]
impl<
        B: Block,
        BE: Backend<B> + 'static,
        KBE: KeystoreBackend,
        C: ClientWithApi<B, BE>,
        N: Network,
    > AsyncProtocol for MpEcdsaKeygenProtocol<B, BE, KBE, C, N>
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
        let protocol_output = Arc::new(tokio::sync::Mutex::new(None));
        let protocol_output_clone = protocol_output.clone();
        let client = self.client.clone();
        let id = self.account_id;
        let logger = self.logger.clone();
        let logger_clone = logger.clone();
        let round_blames = self.round_blames.clone();
        let network = self.network.clone();

        let (i, t, n, mapping) = (
            additional_params.i,
            additional_params.t,
            additional_params.n,
            additional_params.user_id_to_account_id_mapping,
        );

        Ok(JobBuilder::new()
            .protocol(async move {
                let protocol_i = i + 1;
                logger.info(format!(
                    "Starting Keygen Protocol with params: i={protocol_i}, t={t}, n={n}"
                ));
                let keygen = Keygen::new(protocol_i, t, n).map_err(|err| JobError {
                    reason: format!("Keygen setup error: {err:?}"),
                })?;
                let (current_round_blame_tx, current_round_blame_rx) =
                    tokio::sync::watch::channel(CurrentRoundBlame::default());

                round_blames
                    .write()
                    .await
                    .insert(associated_task_id, current_round_blame_rx);

                let (
                    keygen_tx_to_outbound,
                    keygen_rx_async_proto,
                    broadcast_tx_to_outbound,
                    broadcast_rx_from_gadget,
                ) = util::create_job_manager_to_async_protocol_channel_split::<
                    _,
                    Msg<<Keygen as StateMachine>::MessageBody>,
                    PublicKeyGossipMessage,
                >(
                    protocol_message_rx,
                    associated_block_id,
                    associated_retry_id,
                    associated_session_id,
                    associated_task_id,
                    mapping,
                    network,
                );

                let state_machine_wrapper =
                    StateMachineWrapper::new(keygen, current_round_blame_tx, logger.clone());
                logger.debug("Beginning AsyncProtocol - KeyGen");
                let local_key = round_based::AsyncProtocol::new(
                    state_machine_wrapper,
                    keygen_rx_async_proto,
                    keygen_tx_to_outbound,
                )
                .set_watcher(StderrWatcher)
                .run()
                .await
                .map_err(|err| JobError {
                    reason: format!("Keygen protocol error: {err:?}"),
                })?;

                logger.debug("Finished AsyncProtocol - KeyGen");

                let job_result = handle_public_key_gossip(
                    &logger,
                    &local_key,
                    t,
                    i,
                    id,
                    broadcast_tx_to_outbound,
                    broadcast_rx_from_gadget,
                )
                .await?;

                *protocol_output.lock().await = Some((local_key, job_result));
                Ok(())
            })
            .post(async move {
                // Check to see if there is any blame at the end of the protocol
                if let Some(blame) = blame.write().await.remove(&associated_task_id) {
                    let blame = blame.borrow();
                    // TODO: consider the fact that ids from the async protocol are offset by +1
                    if !blame.blamed_parties.is_empty() {
                        logger_clone.error(format!("Blame: {blame:?}"));
                        return Err(JobError {
                            reason: format!("Keygen blame: {blame:?}"),
                        });
                    }
                }

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

async fn handle_public_key_gossip(
    logger: &DebugLogger,
    local_key: &LocalKey<Secp256k1>,
    threshold: u16,
    i: u16,
    my_id: AccountId,
    broadcast_tx_to_outbound: UnboundedSender<PublicKeyGossipMessage>,
    mut broadcast_rx_from_gadget: UnboundedReceiver<PublicKeyGossipMessage>,
) -> Result<JobResult, JobError> {
    let serialized_public_key = local_key.public_key().to_bytes(true).to_vec();
    // Sign the public key with our public key
    // TODO: Either run inside externalities environment, or, use the DKG method
    let signature = my_id
        .sign(KeyTypeId::default(), &serialized_public_key)
        .ok_or_else(|| JobError {
            reason: "Failed to sign public key".to_string(),
        })?
        .0
        .to_vec();

    let mut received_keys = BTreeMap::new();
    received_keys.insert(i, signature.clone());
    let mut received_participants = BTreeMap::new();
    received_participants.insert(i, my_id);

    broadcast_tx_to_outbound
        .send(PublicKeyGossipMessage {
            from: i as _,
            to: None,
            signature,
            id: my_id,
        })
        .map_err(|err| JobError {
            reason: format!("Failed to send public key: {err:?}"),
        })?;

    for _ in 0..threshold {
        let message: PublicKeyGossipMessage =
            broadcast_rx_from_gadget
                .recv()
                .await
                .ok_or_else(|| JobError {
                    reason: "Failed to receive public key".to_string(),
                })?;

        let from = message.from;
        if received_keys.contains_key(&(from as u16)) {
            logger.warn("Received duplicate key");
            continue;
        }

        received_keys.insert(from as u16, message.signature);

        received_participants.insert(from as u16, message.id);
    }

    // Order and collect the map to ensure symmetric submission to blockchain
    let signatures = received_keys
        .into_iter()
        .sorted_by_key(|x| x.0)
        .map(|r| r.1)
        .collect();

    let participants = received_participants
        .into_iter()
        .sorted_by_key(|x| x.0)
        .map(|r| r.1.identity())
        .collect();

    Ok(JobResult::DKGPhaseOne(DKGResult {
        key_type: DkgKeyType::Ecdsa,
        key: serialized_public_key,
        participants,
        threshold: threshold as _,
        signatures,
    }))
}
