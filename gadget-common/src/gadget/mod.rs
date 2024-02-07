use crate::client::{AccountId, ClientWithApi, GadgetJobType, JobsApiForGadget, JobsClient};
use crate::debug_logger::DebugLogger;
use crate::gadget::message::GadgetProtocolMessage;
use crate::gadget::work_manager::WorkManager;
use crate::protocol::{AsyncProtocol, AsyncProtocolRemote};
use crate::Error;
use async_trait::async_trait;
use gadget_core::gadget::substrate::SubstrateGadgetModule;
use gadget_core::job::BuiltExecutableJobWrapper;
use gadget_core::job_manager::{PollMethod, ProtocolWorkManager, WorkManagerInterface};
use network::Network;
use parking_lot::RwLock;
use sc_client_api::{Backend, BlockImportNotification, FinalityNotification};
use sp_api::ProvideRuntimeApi;
use sp_core::keccak_256;
use sp_runtime::traits::{Block, Header};
use sp_runtime::SaturatedConversion;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;
use tangle_primitives::jobs::JobId;
use tangle_primitives::roles::RoleType;

pub mod message;
pub mod network;
pub mod work_manager;

/// Used as a module to place inside the SubstrateGadget
pub struct Module<B, C, N, M, BE> {
    protocol: M,
    network: N,
    job_manager: ProtocolWorkManager<WorkManager>,
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
        M: GadgetProtocol<B, BE, C>,
        BE: Backend<B>,
    > Module<B, C, N, M, BE>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    pub fn new(network: N, module: M, job_manager: ProtocolWorkManager<WorkManager>) -> Self {
        let clock = job_manager.utility.clock.clone();
        Module {
            protocol: module,
            job_manager,
            network,
            clock,
            _pd: Default::default(),
        }
    }
}

pub struct JobInitMetadata<B: Block> {
    pub job_type: GadgetJobType,
    pub role_type: RoleType,
    /// This value only exists if this is a stage2 job
    pub phase1_job: Option<GadgetJobType>,
    pub task_id: <WorkManager as WorkManagerInterface>::TaskID,
    pub retry_id: <WorkManager as WorkManagerInterface>::RetryID,
    pub job_id: JobId,
    pub now: <WorkManager as WorkManagerInterface>::Clock,
    pub at: B::Hash,
}

#[async_trait]
impl<
        C: ClientWithApi<B, BE>,
        B: Block,
        N: Network,
        M: GadgetProtocol<B, BE, C>,
        BE: Backend<B>,
    > SubstrateGadgetModule for Module<B, C, N, M, BE>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
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
        log::info!(target: "gadget", "[{}] Processing finality notification at block number {now}", self.protocol.name());

        let jobs = self
            .protocol
            .client()
            .query_jobs_by_validator(notification.hash, *self.protocol.account_id())
            .await?;

        log::trace!(target: "gadget", "[{}] Found {} jobs for initialization", self.protocol.name(), jobs.len());
        let mut relevant_jobs = Vec::new();

        for job in jobs {
            // Job is expired.
            if job.expiry < now_header {
                log::trace!(target: "gadget", "[{}] The job requested for initialization is expired, skipping submission", self.protocol.name());
                continue;
            }
            // Job is not for this role
            if !self.protocol.role_filter(job.job_type.get_role_type()) {
                log::trace!(target: "gadget", "[{}] The job {} requested for initialization is not for this role {:?}, skipping submission", self.protocol.name(), job.job_id, job.job_type.get_role_type());
                continue;
            }
            // Job is not for this phase
            if !self.protocol.phase_filter(job.job_type.clone()) {
                log::trace!(target: "gadget", "[{}] The job {} requested for initialization is not for this phase {:?}, skipping submission", self.protocol.name(), job.job_id, job.job_type);
                continue;
            }

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

            let phase1_job = if job.job_type.is_phase_one() {
                None
            } else {
                let phase_one_job_id = job
                    .job_type
                    .get_phase_one_id()
                    .expect("Should exist for non phase 1 jobs");
                let phase1_job = self
                        .protocol
                        .client()
                        .query_job_result(notification.hash, job.job_type.get_role_type(), phase_one_job_id)
                        .await?
                        .ok_or_else(|| Error::ClientError {
                            err: format!("Corresponding phase one job {phase_one_job_id} not found for phase two job {job_id}"),
                        })?;
                Some(phase1_job.job_type)
            };

            relevant_jobs.push(JobInitMetadata {
                role_type: job.job_type.get_role_type(),
                job_type: job.job_type,
                phase1_job,
                task_id,
                retry_id,
                now,
                job_id,
                at: notification.hash,
            });
        }

        for relevant_job in relevant_jobs {
            let task_id = relevant_job.task_id;
            let retry_id = relevant_job.retry_id;
            log::trace!(target: "gadget", "[{}] Creating job for task {task_id} with retry id {retry_id}", self.protocol.name(), task_id = hex::encode(task_id), retry_id = retry_id);
            match self.protocol.create_next_job(relevant_job).await {
                Ok(params) => {
                    match self
                        .protocol
                        .create(0, now, retry_id, task_id, params)
                        .await
                    {
                        Ok(job) => {
                            let (remote, protocol) = job;
                            if let Err(err) = self.job_manager.push_task(
                                task_id,
                                false,
                                Arc::new(remote),
                                protocol,
                            ) {
                                self.protocol
                                    .process_error(
                                        Error::WorkManagerError { err },
                                        &self.job_manager,
                                    )
                                    .await;
                            }
                        }

                        Err(err) => {
                            self.protocol
                                .logger()
                                .error(format!("Failed to create async protocol: {err:?}"));
                        }
                    }
                }

                Err(Error::ParticipantNotSelected { id, reason }) => {
                    log::debug!(target: "gadget", "[{}] Participant {id} not selected for job {task_id} with retry id {retry_id} because {reason}", self.protocol.name(), id = id, task_id = hex::encode(task_id), retry_id = retry_id, reason = reason);
                }

                Err(err) => {
                    self.protocol
                        .logger()
                        .error(format!("Failed to generate job parameters: {err:?}"));
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
pub trait GadgetProtocol<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>>:
    AsyncProtocol + Send + Sync
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    /// Given an input of a valid and relevant job, return the parameters needed to start the async protocol
    /// Note: the parameters returned must be relevant to the `AsyncProtocol` implementation of this protocol
    ///
    /// In case the participant is not selected for some reason, return an [`Error::ParticipantNotSelected`]
    async fn create_next_job(
        &self,
        job: JobInitMetadata<B>,
    ) -> Result<<Self as AsyncProtocol>::AdditionalParams, Error>;

    /// Process a block import notification
    async fn process_block_import_notification(
        &self,
        notification: BlockImportNotification<B>,
        job_manager: &ProtocolWorkManager<WorkManager>,
    ) -> Result<(), Error>;
    /// Process an error that may arise from the work manager, async protocol, or the executor
    async fn process_error(&self, error: Error, job_manager: &ProtocolWorkManager<WorkManager>);
    /// The account ID of this node. Jobs queried will be filtered by this account ID
    fn account_id(&self) -> &AccountId;

    /// The Protocol Name.
    /// Used for logging and debugging purposes
    fn name(&self) -> String;
    /// Filter queried jobs by role type.
    /// ## Example
    ///
    /// ```rust,ignore
    /// fn role_filter(&self, role: RoleType) -> bool {
    ///   matches!(role, RoleType::Tss(ThresholdSignatureRoleType::ZengoGG20Secp256k1))
    /// }
    /// ```
    fn role_filter(&self, role: RoleType) -> bool;

    /// Filter queried jobs by Job type & Phase.
    /// ## Example
    ///
    /// ```rust,ignore
    /// fn phase_filter(&self, job: JobType<AccountId>) -> bool {
    ///   matches!(job, JobType::DKGTSSPhaseOne(_))
    /// }
    /// ```
    fn phase_filter(&self, job: JobType<AccountId>) -> bool;
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
