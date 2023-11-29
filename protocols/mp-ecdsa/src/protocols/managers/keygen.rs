#![allow(clippy::needless_return)]

use crate::client::{ClientWithApi, MpEcdsaClient};
use crate::util::DebugLogger;
use gadget_core::job::{BuiltExecutableJobWrapper, JobBuilder};
use gadget_core::job_manager::{JobMetadata, ProtocolWorkManager, WorkManagerInterface};
use parking_lot::RwLock;
use std::error::Error;
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use dkg_runtime_primitives::GENESIS_AUTHORITY_SET_ID;
use sc_client_api::Backend;
use sp_runtime::SaturatedConversion;
use sp_runtime::traits::Header;
use webb_gadget::gadget::work_manager::WebbWorkManager;
use webb_gadget::protocol::AsyncProtocolRemote;
use webb_gadget::Block;
use webb_gadget::gadget::Job;
use crate::client::types::AnticipatedKeygenExecutionStatus;
use crate::indexing::KeygenPartyId;

/// The KeygenManager is an abstraction that manages the lifecycle for executing and maintaining
/// keygen protocols
#[derive(Clone)]
pub struct KeygenManager<B, BE, C> {
    logger: DebugLogger,
    work_manager: ProtocolWorkManager<WebbWorkManager>,
    active_keygen_retry_id: Arc<AtomicUsize>,
    keygen_state: Arc<RwLock<KeygenState>>,
    latest_executed_session_id:
        Arc<RwLock<Option<<WebbWorkManager as WorkManagerInterface>::SessionID>>>,
    client: MpEcdsaClient<B, BE, C>,
    pub finished_count: Arc<AtomicUsize>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
/// State of the KeygenManager
pub enum KeygenState {
    Uninitialized,
    RunningKeygen,
    RunningGenesisKeygen,
    // session_completed denotes the session that executed the keygen, NOT
    // the generated DKG public key for the next session
    KeygenCompleted { session_completed: u64 },
    Failed { session_id: u64 },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum KeygenRound {
    Genesis,
    Next,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ProtoStageType {
    KeygenGenesis,
    KeygenStandard,
}

impl<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>> KeygenManager<B, BE, C> {
    pub fn new(
        logger: DebugLogger,
        client: MpEcdsaClient<B, BE, C>,
        work_manager: ProtocolWorkManager<WebbWorkManager>,
    ) -> Self {
        Self {
            logger,
            work_manager,
            active_keygen_retry_id: Arc::new(AtomicUsize::new(0)),
            keygen_state: Arc::new(RwLock::new(KeygenState::Uninitialized)),
            latest_executed_session_id: Arc::new(RwLock::new(None)),
            finished_count: Arc::new(AtomicUsize::new(0)),
            client,
        }
    }

    pub fn deliver_message(
        &self,
        message: <WebbWorkManager as WorkManagerInterface>::ProtocolMessage,
    ) {
        if let Err(err) = self.work_manager.deliver_message(message) {
            self.logger
                .error(format!("Error delivering message: {err:?}"));
        }
    }

    pub fn session_id_of_active_keygen(
        &self,
        now: <WebbWorkManager as WorkManagerInterface>::Clock,
    ) -> Option<JobMetadata<WebbWorkManager>> {
        self.work_manager.get_active_sessions_metadata(now).pop()
    }

    pub fn get_latest_executed_session_id(
        &self,
    ) -> Option<<WebbWorkManager as WorkManagerInterface>::SessionID> {
        *self.latest_executed_session_id.read()
    }

    fn state(&self) -> KeygenState {
        *self.keygen_state.read()
    }

    pub fn set_state(&self, state: KeygenState) {
        *self.keygen_state.write() = state;
    }

    /// GENERAL WORKFLOW for Keygen
    ///
    /// Session 0 (beginning): immediately run genesis
    /// Session 0 (ending): run keygen for session 1
    /// Session 1 (ending): run keygen for session 2
    /// Session 2 (ending): run keygen for session 3
    /// Session 3 (ending): run keygen for session 4
    pub async fn on_block_finalized(&self, header: &B::Header) -> Option<Job> {
        if let Some((active, _queued)) = self.client.validator_set(header).await {
            // Poll to clear any tasks that have finished and to make room for a new potential
            // keygen
            self.work_manager.poll();
            let now_n = *header.number();
            let block_id: u64 = now_n.saturated_into();
            let session_id = active.id;
            let current_protocol = self.session_id_of_active_keygen(now_n);
            let state = self.state();
            let anticipated_execution_status = self.client.should_execute_new_keygen(header).await;
            let executed_count = self.finished_count.load(Ordering::SeqCst);

            self.logger.debug(format!(
                "*** KeygenManager on_block_finalized: session={session_id},block={block_id}, state={state:?}, current_protocol={current_protocol:?} | total executed: {executed_count}",
            ));
            self.logger.debug(format!(
                "*** Should execute new keygen? {anticipated_execution_status:?}"
            ));

            // If a keygen is already running (and isn't stalled), don't start another one
            if self.valid_keygen_running(&current_protocol) {
                return None
            }

            // Always perform pre-checks
            if let Some(job) = self
                .pre_checks(
                    session_id,
                    state,
                    header,
                    &anticipated_execution_status,
                )
                .await
            {
                return Some(job)
            }


            if session_id == GENESIS_AUTHORITY_SET_ID {
                self.genesis_checks(state, header, &anticipated_execution_status)
                    .await
            } else {
                self.next_checks(
                    session_id,
                    state,
                    header,
                    &anticipated_execution_status,
                )
                .await
            }
        } else {
            None
        }
    }

    fn valid_keygen_running(&self, current_protocol: &Option<JobMetadata<WebbWorkManager>>) -> bool {
        // If a keygen is already running (and isn't stalled), don't start another one
        if let Some(current_protocol) = current_protocol.as_ref() {
            if current_protocol.is_active && !current_protocol.is_stalled {
                self
                    .logger
                    .info("Will not trigger a keygen since one is already running");
                return true;
            }
        }

        false
    }

    /// Check to see if a genesis keygen failed, or, if we are already running a non-stalled
    /// protocol
    async fn pre_checks(
        &self,
        session_id: u64,
        state: KeygenState,
        header: &B::Header,
        anticipated_execution: &AnticipatedKeygenExecutionStatus,
    ) -> Option<Job> {
        if anticipated_execution.force_execute {
            // Unconditionally execute another keygen, overwriting the previous one if necessary
            let stage = if session_id == GENESIS_AUTHORITY_SET_ID
                && self.finished_count.load(Ordering::SeqCst) == 0
            {
                KeygenRound::Genesis
            } else {
                KeygenRound::Next
            };

            return self.maybe_start_keygen_for_stage(stage, header, anticipated_execution)
                .await;
        }

        // It's possible genesis failed and we need to retry
        if session_id == GENESIS_AUTHORITY_SET_ID
            && matches!(state, KeygenState::Failed { session_id: 0 })
            && self.client.dkg_pub_key_is_unset(header).await
        {
            self
                .logger
                .warn("We will trigger another genesis keygen because the previous one failed");
            return self.maybe_start_keygen_for_stage(
                KeygenRound::Genesis,
                header,
                anticipated_execution,
            )
            .await;
        }

        None
    }

    /// Check to see if we need to run a genesis keygen (session = 0), or, a keygen for session 1
    async fn genesis_checks(
        &self,
        state: KeygenState,
        header: &B::Header,
        anticipated_execution: &AnticipatedKeygenExecutionStatus,
    ) -> Option<Job> {
        if state == KeygenState::Uninitialized {
            // If we are at genesis, and there is no active keygen, create and immediately
            // start() one
            return self
                .maybe_start_keygen_for_stage(
                    KeygenRound::Genesis,
                    header,
                    anticipated_execution,
                )
                .await;
        }

        if state == KeygenState::RunningGenesisKeygen {
            // If we are at genesis, and a genesis keygen is running, do nothing
            return None
        }

        if state == KeygenState::RunningKeygen {
            // If we are at genesis, and there is a next keygen running, do nothing
            return None
        }

        if matches!(
            state,
            KeygenState::KeygenCompleted {
                session_completed: 0
            }
        ) {
            // If we are at genesis, and we have completed keygen, we may need to begin a keygen
            // for session 1
            return self
                .maybe_start_keygen_for_stage(
                    KeygenRound::Next,
                    header,
                    anticipated_execution,
                )
                .await;
        }

        if matches!(state, KeygenState::Failed { session_id: 1 }) {
            // If we are at genesis, and we have failed keygen for session 1, we may need to begin a
            // keygen for session 1
            return self
                .maybe_start_keygen_for_stage(
                    KeygenRound::Next,
                    header,
                    anticipated_execution,
                )
                .await;
        }

        None
    }

    /// Check to see if we need to run a keygen for session 2, 3, 4, .., etc.
    async fn next_checks(
        &self,
        session_id: u64,
        state: KeygenState,
        header: &B::Header,
        anticipated_execution: &AnticipatedKeygenExecutionStatus,
    ) -> Option<Job> {
        // Check bad states. These should never happen in a well-behaved program
        if state == KeygenState::RunningGenesisKeygen {
            self.logger.error(format!("Invalid keygen manager state: {session_id} > GENESIS_AUTHORITY_SET_ID && {state:?} == KeygenState::GenesisKeygenCompleted || {state:?} == KeygenState::RunningGenesisKeygen"));
            return None;
        }

        if state == KeygenState::Uninitialized {
            // We joined the network after genesis. We need to start a keygen for session `now`,
            // so long as the next pub key isn't already on chain
            if self.client.get_next_dkg_pub_key(header).await.is_none() {
                return self.maybe_start_keygen_for_stage(
                    KeygenRound::Next,
                    header,
                    anticipated_execution,
                )
                .await;
            }
        }

        if state == KeygenState::RunningKeygen {
            // We are in the middle of a keygen. Do nothing
            return None;
        }

        if matches!(
            state,
            KeygenState::KeygenCompleted { .. } | KeygenState::Failed { .. }
        ) {
            // We maybe need to start a keygen for session `session_id`:
            return self
                .maybe_start_keygen_for_stage(
                    KeygenRound::Next,
                    header,
                    anticipated_execution,
                )
                .await;
        }

        None
    }

    async fn maybe_start_keygen_for_stage(
        &self,
        stage: KeygenRound,
        header: &B::Header,
        anticipated_execution_status: &AnticipatedKeygenExecutionStatus,
    ) -> Option<Job> {
        let authority_set = if let Some((active, queued)) = self.client.validator_set(header).await {
            match stage {
                KeygenRound::Genesis => active,
                KeygenRound::Next => queued,
            }
        } else {
            return None
        };

        let session_id = authority_set.id;
        self.logger.debug(format!(
            "Will attempt to start keygen for session {session_id}"
        ));

        if stage != KeygenRound::Genesis {
            // We need to ensure session progress is close enough to the end to begin execution
            if !anticipated_execution_status.execute && !anticipated_execution_status.force_execute
            {
                self
                    .logger
                    .debug("🕸️  Not executing new keygen protocol");
                return None
            }

            if self.client.get_next_dkg_pub_key(header).await.is_some()
                && !anticipated_execution_status.force_execute
            {
                self.logger.debug("🕸Not executing new keygen protocol because we already have a next DKG public key");
                return None
            }

            if self.finished_count.load(Ordering::SeqCst) != session_id as usize {
                self
                    .logger
                    .warn("We have already run this protocol, is this a re-try?");
            }
        } else {
            // if we are in genesis, make sure that the current public key isn't already on-chain
            if !self.client.dkg_pub_key_is_unset(header).await {
                self.logger.debug(
                    "🕸️  Not executing new keygen protocol because we already have a DKG public key",
                );
                return None
            }
        }

        let party_idx = match stage {
            KeygenRound::Genesis => self.client.get_party_index(header).await,
            KeygenRound::Next => self.client.get_next_party_index(header).await,
        };

        let threshold = match stage {
            KeygenRound::Genesis => self.client.get_signature_threshold(header).await,
            KeygenRound::Next => self.client.get_next_signature_threshold(header).await,
        };

        // Check whether the worker is in the best set or return
        let party_i = match party_idx {
            Some(party_index) => {
                self.logger.info(format!("🕸️  PARTY {party_index} | SESSION {session_id} | IN THE SET OF BEST AUTHORITIES: session: {session_id} | threshold: {threshold}"));
                if let Ok(res) = KeygenPartyId::try_from(party_index) {
                    res
                } else {
                    return None
                }
            }
            None => {
                self.logger.info(format!(
                    "🕸️  NOT IN THE SET OF BEST AUTHORITIES: session: {session_id}"
                ));
                return None
            }
        };

        let best_authorities = match stage {
            KeygenRound::Genesis => self.client.get_best_authorities(header).await,
            KeygenRound::Next => self.client.get_next_best_authorities(header).await,
        };

        let best_authorities = best_authorities
            .into_iter()
            .flat_map(|(i, p)| KeygenPartyId::try_from(i).map(|i| (i, p)))
            .collect();

        let authority_public_key = self.client.get_authority_public_key();
        let proto_stage_ty = if stage == KeygenRound::Genesis {
            ProtoStageType::KeygenGenesis
        } else {
            ProtoStageType::KeygenStandard
        };

        self.logger.debug(format!("🕸️  PARTY {party_i} | SPAWNING KEYGEN SESSION {session_id} | BEST AUTHORITIES: {best_authorities:?}"));

        let keygen_protocol_hash = get_keygen_protocol_hash(
            session_id,
            self.active_keygen_retry_id.load(Ordering::SeqCst),
        );

        // If we are starting this keygen because of an emergency keygen, clear the unsigned
        // proposals locally
        if anticipated_execution_status.force_execute {
            self.signing_manager.clear_enqueued_proposal_tasks();
        }

        // For now, always use the MpEcdsa variant
        let params = KeygenProtocolSetupParameters::MpEcdsa {
            best_authorities,
            authority_public_key,
            party_i,
            session_id,
            associated_block: *header.number(),
            threshold,
            stage: proto_stage_ty,
            keygen_protocol_hash,
        };

        let dkg = dkg_worker
            .dkg_modules
            .get_keygen_protocol(&params)
            .expect("Default should be present");

        if let Some((handle, task)) = dkg.initialize_keygen_protocol(params).await {
            // Before sending the task, force clear all previous tasks to allow the new one
            // the immediately run
            if anticipated_execution_status.force_execute {
                self.logger.debug(
                    "🕸️  PARTY {party_i} | SPAWNING KEYGEN SESSION {session_id} | FORCE EXECUTE",
                );
                self.work_manager.force_shutdown_all();
            }

            // Now, map the task to properly handle cleanup
            let logger = self.logger.clone();
            let signing_manager = self.signing_manager.clone();
            let keygen_manager = self.clone();
            let err_handler_tx = dkg_worker.error_handler_channel.tx.clone();

            // Prevent any signing tasks from running while the keygen is running
            signing_manager.keygen_lock();

            let task = JobBuilder::new()
                .build(async move {
                match task.await {
                    Ok(_) => {
                        keygen_manager.set_state(KeygenState::KeygenCompleted {
                            session_completed: session_id,
                        });
                        let _ = keygen_manager.finished_count.fetch_add(1, Ordering::SeqCst);
                        signing_manager.keygen_unlock();
                        logger
                            .info("The keygen meta handler has executed successfully".to_string());

                        Ok(())
                    }

                    Err(err) => {
                        logger.error(format!("Error executing meta handler {:?}", &err));
                        keygen_manager.set_state(KeygenState::Failed { session_id });
                        signing_manager.keygen_unlock();
                        let _ = err_handler_tx.send(err.clone());
                        Err(err)
                    }
                }
            });

            // Update states
            match stage {
                KeygenRound::Genesis => self.set_state(KeygenState::RunningGenesisKeygen),
                KeygenRound::Next => self.set_state(KeygenState::RunningKeygen),
            }

            *self.latest_executed_session_id
                .write() = Some(session_id);

            Some((handle, task))
        } else {
            None
        }
    }

    /// Pushes a task to the work manager, manually polling and starting the keygen protocol
    pub fn push_task(
        &self,
        handle: AsyncProtocolRemote,
        task: BuiltExecutableJobWrapper,
    ) -> Result<(), Box<dyn Error>> {
        let task_hash = get_keygen_protocol_hash(
            handle.session_id,
            self.active_keygen_retry_id.load(Ordering::Relaxed),
        );
        self.work_manager
            .push_task(task_hash, false, handle, task)?;
        // poll to start the task
        self.work_manager.poll();
        Ok(())
    }
}

/// Computes keccak_256(session ID || retry_id)
fn get_keygen_protocol_hash(
    session_id: <WebbWorkManager as WorkManagerInterface>::SessionID,
    active_keygen_retry_id: usize,
) -> [u8; 32] {
    let mut session_id_bytes = session_id.to_be_bytes().to_vec();
    let retry_id_bytes = active_keygen_retry_id.to_be_bytes();
    session_id_bytes.extend_from_slice(&retry_id_bytes);
    dkg_runtime_primitives::keccak_256(&session_id_bytes)
}
