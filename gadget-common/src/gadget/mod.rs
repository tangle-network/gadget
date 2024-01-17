use crate::client::{AccountId, ClientWithApi, JobsClient};
use crate::debug_logger::DebugLogger;
use crate::gadget::message::GadgetProtocolMessage;
use crate::gadget::work_manager::WebbWorkManager;
use crate::protocol::AsyncProtocolRemote;
use crate::Error;
use async_trait::async_trait;
use gadget_core::gadget::substrate::SubstrateGadgetModule;
use gadget_core::job::BuiltExecutableJobWrapper;
use gadget_core::job_manager::{PollMethod, ProtocolWorkManager, WorkManagerInterface};
use network::Network;
use pallet_jobs_rpc_runtime_api::JobsApi;
use parking_lot::RwLock;
use sc_client_api::{Backend, BlockImportNotification, FinalityNotification};
use sp_api::ProvideRuntimeApi;
use sp_core::keccak_256;
use sp_runtime::traits::{Block, Header};
use sp_runtime::SaturatedConversion;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;
use tangle_primitives::jobs::{JobId, JobType};
use tangle_primitives::roles::RoleType;

pub mod message;
pub mod network;
pub mod work_manager;

/// Used as a module to place inside the SubstrateGadget
pub struct WebbModule<B, C, N, M, BE> {
    protocol: M,
    network: N,
    job_manager: ProtocolWorkManager<WebbWorkManager>,
    clock: Arc<RwLock<Option<u64>>>,
    _pd: PhantomData<(B, C, BE)>,
}

const DEFAULT_MAX_ACTIVE_TASKS: usize = 4;
const DEFAULT_MAX_PENDING_TASKS: usize = 4;
const DEFAULT_POLL_INTERVAL: Option<Duration> = Some(Duration::from_millis(200));

#[derive(Debug)]
pub struct WorkManagerConfig {
    pub interval: Option<Duration>,
    pub max_active_tasks: usize,
    pub max_pending_tasks: usize,
}

impl<
        C: ClientWithApi<B, BE>,
        B: Block,
        N: Network,
        M: WebbGadgetProtocol<B, BE, C>,
        BE: Backend<B>,
    > WebbModule<B, C, N, M, BE>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    pub fn new(network: N, module: M, job_manager: ProtocolWorkManager<WebbWorkManager>) -> Self {
        let clock = job_manager.utility.clock.clone();
        WebbModule {
            protocol: module,
            job_manager,
            network,
            clock,
            _pd: Default::default(),
        }
    }
}

pub struct JobInitMetadata {
    pub job_type: JobType<AccountId>,
    /// This value only exists if this is a stage2 job
    pub phase1_participants: Option<Vec<AccountId>>,
    /// This value only exists if this is a stage2 job that requires a threshold
    pub phase1_threshold: Option<u16>,
    pub task_id: <WebbWorkManager as WorkManagerInterface>::TaskID,
    pub retry_id: <WebbWorkManager as WorkManagerInterface>::RetryID,
    pub job_id: JobId,
    pub now: <WebbWorkManager as WorkManagerInterface>::Clock,
}

#[async_trait]
impl<
        C: ClientWithApi<B, BE>,
        B: Block,
        N: Network,
        M: WebbGadgetProtocol<B, BE, C>,
        BE: Backend<B>,
    > SubstrateGadgetModule for WebbModule<B, C, N, M, BE>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    type Error = Error;
    type ProtocolMessage = GadgetProtocolMessage;
    type Block = B;
    type Client = C;

    async fn get_next_protocol_message(&self) -> Option<Self::ProtocolMessage> {
        self.network.next_message().await
    }

    async fn process_finality_notification(
        &self,
        notification: FinalityNotification<B>,
    ) -> Result<(), Self::Error> {
        let now_header = *notification.header.number();
        let now: u64 = now_header.saturated_into();
        *self.clock.write() = Some(now);

        let jobs = self
            .protocol
            .client()
            .query_jobs_by_validator(notification.hash, self.protocol.account_id().clone())
            .await?;

        let role_type = self.protocol.role_type();

        let mut relevant_jobs = Vec::new();

        for job in jobs {
            if job.expiry > now_header {
                if job.job_type.get_role_type() == role_type {
                    let job_id = job.job_id;
                    let task_id = job_id.to_be_bytes();
                    let task_id = keccak_256(&task_id);
                    if self.job_manager.job_exists(&task_id) {
                        // log::warn!(target: "gadget", "The job requested for initialization is already running or enqueued, skipping submission");
                        continue;
                    }

                    let retry_id = self
                        .job_manager
                        .latest_retry_id(&task_id)
                        .map(|r| r + 1)
                        .unwrap_or(0);

                    if job.job_type.is_phase_one() && self.protocol.is_phase_one() {
                        let participants = job
                            .job_type
                            .clone()
                            .get_participants()
                            .expect("Should exist for all stage 1 jobs");
                        if participants.contains(self.protocol.account_id()) {
                            relevant_jobs.push(JobInitMetadata {
                                job_type: job.job_type,
                                // Only supply for stage 2 jobs
                                phase1_participants: None,
                                phase1_threshold: None,
                                task_id,
                                retry_id,
                                now,
                                job_id,
                            });
                            continue;
                        }
                    }

                    if !job.job_type.is_phase_one() && !self.protocol.is_phase_one() {
                        let previous_job_id = job
                            .job_type
                            .get_phase_one_id()
                            .expect("Should exist for stage 2 jobs");
                        let phase1_job = self
                            .protocol
                            .client()
                            .query_job_result(notification.hash, role_type.clone(), previous_job_id)
                            .await?
                            .ok_or_else(|| Error::ClientError {
                                err: format!("Corresponding phase one job {previous_job_id} not found for phase two job {job_id}"),
                            })?;

                        let participants = phase1_job
                            .job_type
                            .clone()
                            .get_participants()
                            .expect("Should exist for stage 1 signing");
                        let threshold = phase1_job
                            .job_type
                            .clone()
                            .get_threshold()
                            .expect("T should exist for stage 1 signing")
                            as u16;

                        if participants.contains(self.protocol.account_id()) {
                            relevant_jobs.push(JobInitMetadata {
                                job_type: job.job_type,
                                phase1_participants: Some(participants),
                                phase1_threshold: Some(threshold),
                                task_id,
                                job_id,
                                retry_id,
                                now,
                            });

                            continue;
                        }
                    }
                }
            }
        }

        for relevant_job in relevant_jobs {
            let task_id = relevant_job.task_id;
            match self.protocol.create_next_job(relevant_job).await {
                Ok(job) => {
                    let (remote, protocol) = job;
                    if let Err(err) =
                        self.job_manager
                            .push_task(task_id, false, Arc::new(remote), protocol)
                    {
                        self.protocol
                            .process_error(Error::WorkManagerError { err }, &self.job_manager)
                            .await;
                    }
                }

                Err(err) => {
                    log::error!(target: "gadget", "Failed to process job: {err:?}");
                }
            }
        }

        // Poll jobs on each finality notification if we're using manual polling.
        // This helps synchronize the actions of nodes in the network
        if self.job_manager.poll_method() == PollMethod::Manual {
            self.job_manager.poll();
        }

        Ok(())
    }

    async fn process_block_import_notification(
        &self,
        notification: BlockImportNotification<B>,
    ) -> Result<(), Self::Error> {
        self.protocol
            .process_block_import_notification(notification, &self.job_manager)
            .await
    }

    async fn process_protocol_message(
        &self,
        message: Self::ProtocolMessage,
    ) -> Result<(), Self::Error> {
        self.job_manager
            .deliver_message(message)
            .map(|_| ())
            .map_err(|err| Error::WorkManagerError { err })
    }

    async fn process_error(&self, error: Self::Error) {
        self.protocol.process_error(error, &self.job_manager).await
    }
}

pub type Job = (AsyncProtocolRemote, BuiltExecutableJobWrapper);

#[async_trait]
pub trait WebbGadgetProtocol<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>>:
    Send + Sync
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    async fn create_next_job(&self, job: JobInitMetadata) -> Result<Job, Error>;
    async fn process_block_import_notification(
        &self,
        notification: BlockImportNotification<B>,
        job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) -> Result<(), Error>;
    async fn process_error(&self, error: Error, job_manager: &ProtocolWorkManager<WebbWorkManager>);
    fn account_id(&self) -> &AccountId;
    fn role_type(&self) -> RoleType;
    fn is_phase_one(&self) -> bool;
    fn client(&self) -> &JobsClient<B, BE, C>;
    fn logger(&self) -> &DebugLogger;
    fn get_work_manager_config(&self) -> WorkManagerConfig {
        WorkManagerConfig {
            interval: DEFAULT_POLL_INTERVAL,
            max_active_tasks: DEFAULT_MAX_ACTIVE_TASKS,
            max_pending_tasks: DEFAULT_MAX_PENDING_TASKS,
        }
    }
}
