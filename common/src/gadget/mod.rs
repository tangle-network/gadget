use crate::client::{ClientWithApi, JobsClient};
use crate::debug_logger::DebugLogger;
use crate::gadget::work_manager::WorkManager;
use crate::protocol::{AsyncProtocol, AsyncProtocolRemote};
use crate::tangle_runtime::*;
use crate::Error;
use async_trait::async_trait;
use gadget_core::gadget::general::GadgetWithClient;
use gadget_core::gadget::manager::AbstractGadget;
use gadget_core::job::{BuiltExecutableJobWrapper, ExecutableJob, JobBuilder};
use gadget_core::job_manager::{ProtocolWorkManager, WorkManagerInterface};
use network::Network;
use parking_lot::{Mutex, RwLock};
use sp_core::sr25519;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;
use substrate::JobInitMetadata;

pub mod core;
pub mod message;
pub mod metrics;
pub mod network;
#[cfg(feature = "substrate")]
pub mod substrate;
pub mod work_manager;

/// Used as a module to place inside the SubstrateGadget
pub struct GeneralModule<C, N, M, Event, ProtocolMessage, Error> {
    protocol: M,
    network: N,
    job_manager: ProtocolWorkManager<WorkManager>,
    clock: Arc<RwLock<Option<u64>>>,
    event_handler: Box<dyn EventHandler<C, N, M, Event, Error>>,
    _client: PhantomData<ProtocolMessage>,
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

impl Default for WorkManagerConfig {
    fn default() -> Self {
        WorkManagerConfig {
            interval: DEFAULT_POLL_INTERVAL,
            max_active_tasks: DEFAULT_MAX_ACTIVE_TASKS,
            max_pending_tasks: DEFAULT_MAX_PENDING_TASKS,
        }
    }
}

impl<
        C: ClientWithApi<Event>,
        N: Network<ProtocolMessage>,
        M: GadgetProtocol<Self, C>,
        Event,
        ProtocolMessage,
        Error,
    > GeneralModule<C, N, M, Event, ProtocolMessage, Error>
where
    Event: Send + Sync + 'static,
    ProtocolMessage: Send + Sync + 'static,
    Error: std::error::Error + Send + Sync + 'static,
{
    pub fn new<Evt: EventHandler<C, N, M, Event, Error>>(
        network: N,
        module: M,
        job_manager: ProtocolWorkManager<WorkManager>,
        event_handler: Evt,
    ) -> Self {
        let clock = job_manager.utility.clock.clone();
        GeneralModule {
            protocol: module,
            job_manager,
            event_handler: Box::new(event_handler),
            network,
            clock,
            _client: Default::default(),
        }
    }
}

#[async_trait]
impl<
        C: ClientWithApi<Event>,
        N: Network<ProtocolMessage>,
        M: GadgetProtocol<Self, C>,
        Event,
        ProtocolMessage,
        Error,
    > AbstractGadget for GeneralModule<C, N, M, Event, ProtocolMessage, Error>
where
    Self: GadgetWithClient<ProtocolMessage, Event, Error>,
    Event: Send + Sync + 'static,
    ProtocolMessage: Send + Sync + 'static,
    Error: std::error::Error + Send + Sync + 'static,
{
    type Event = Event;
    type ProtocolMessage = ProtocolMessage;
    type Error = Error;

    async fn next_event(&self) -> Option<Self::Event> {
        self.protocol.client().client.next_event().await
    }

    async fn get_next_protocol_message(&self) -> Option<Self::ProtocolMessage> {
        <Self as GadgetWithClient<ProtocolMessage, Event, Error>>::get_next_protocol_message(self)
            .await
    }

    async fn on_event_received(&self, notification: Self::Event) -> Result<(), Self::Error> {
        self.event_handler.process_event(notification).await
    }

    async fn process_protocol_message(
        &self,
        message: Self::ProtocolMessage,
    ) -> Result<(), Self::Error> {
        <Self as GadgetWithClient<ProtocolMessage, Event, Error>>::process_protocol_message(
            self, message,
        )
        .await
    }

    async fn process_error(&self, error: Self::Error) {
        <Self as GadgetWithClient<ProtocolMessage, Event, Error>>::process_error(self, error).await
    }
}

#[async_trait]
pub trait EventHandler<C, N, M, Event, Error>: Send + Sync + 'static {
    async fn process_event(&self, notification: Event) -> Result<(), Error>;
}

/*
#[async_trait]
impl<Event, Error, ProtocolMessage, C: ClientWithApi<Event>, N: Network<ProtocolMessage>, M: GadgetProtocol<Self, C>> GadgetWithClient<ProtocolMessage, Event, Error> for GeneralModule<C, N, M, Event, ProtocolMessage, Error>
    where Self: AbstractGadget<Event = Event, Error = Error, ProtocolMessage = ProtocolMessage>{

    type Client = C;

    async fn get_next_protocol_message(&self) -> Option<ProtocolMessage> {
        self.network.next_message().await
    }

    async fn process_event(&self, notification: Event) -> Result<(), Error> {
        self.event_handler.process_event(notification).await
    }

    async fn process_protocol_message(
        &self,
        message: ProtocolMessage,
    ) -> Result<(), crate::Error> {
        self.job_manager
            .deliver_message(message)
            .map(|_| ())
            .map_err(|err| crate::Error::WorkManagerError { err })
    }

    async fn process_error(&self, error: crate::Error) {
        self.protocol.process_error(error, &self.job_manager).await
    }
}*/

pub type Job = (AsyncProtocolRemote, BuiltExecutableJobWrapper);

#[async_trait]
pub trait GadgetProtocol<Event: Send + Sync + 'static, C: ClientWithApi<Event>>:
    AsyncProtocol + Send + Sync
{
    /// Given an input of a valid and relevant job, return the parameters needed to start the async protocol
    /// Note: the parameters returned must be relevant to the `AsyncProtocol` implementation of this protocol
    ///
    /// In case the participant is not selected for some reason, return an [`Error::ParticipantNotSelected`]
    async fn create_next_job(
        &self,
        job: JobInitMetadata,
        work_manager: &ProtocolWorkManager<WorkManager>,
    ) -> Result<<Self as AsyncProtocol>::AdditionalParams, Error>;

    /// Process an error that may arise from the work manager, async protocol, or the executor
    async fn process_error(&self, error: Error, job_manager: &ProtocolWorkManager<WorkManager>);
    /// The account ID of this node. Jobs queried will be filtered by this account ID
    fn account_id(&self) -> &sr25519::Public;

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
    fn role_filter(&self, role: roles::RoleType) -> bool;

    /// Filter queried jobs by Job type & Phase.
    /// ## Example
    ///
    /// ```rust,ignore
    /// fn phase_filter(&self, job: JobType<AccountId, MaxParticipants, MaxSubmissionLen>) -> bool {
    ///   matches!(job, JobType::DKGTSSPhaseOne(_))
    /// }
    /// ```
    fn phase_filter(
        &self,
        job: jobs::JobType<AccountId32, MaxParticipants, MaxSubmissionLen, MaxAdditionalParamsLen>,
    ) -> bool;
    fn client(&self) -> JobsClient<C, Event>;
    fn logger(&self) -> DebugLogger;
    fn get_work_manager_config(&self) -> WorkManagerConfig {
        Default::default()
    }
}

trait MetricizedJob: ExecutableJob {
    fn with_metrics(self) -> BuiltExecutableJobWrapper
    where
        Self: Sized,
    {
        let job = Arc::new(tokio::sync::Mutex::new(self));
        let job2 = job.clone();
        let job3 = job.clone();
        let job4 = job.clone();
        let tokio_metrics = tokio::runtime::Handle::current().metrics();
        crate::prometheus::TOKIO_ACTIVE_TASKS.set(tokio_metrics.active_tasks_count() as f64);
        let now_init = Arc::new(Mutex::new(None));
        let now_clone = now_init.clone();
        let now_clone2 = now_init.clone();

        JobBuilder::default()
            .pre(async move {
                now_init.lock().replace(std::time::Instant::now());
                job.lock().await.pre_job_hook().await
            })
            .protocol(async move {
                // At this point, we know the job will be executed
                crate::prometheus::JOBS_STARTED.inc();
                crate::prometheus::JOBS_RUNNING.inc();
                job2.lock().await.job().await
            })
            .post(async move {
                // Run the job's post hook
                job3.lock().await.post_job_hook().await?;
                crate::prometheus::JOBS_COMPLETED_SUCCESS.inc();
                crate::prometheus::JOBS_RUNNING.dec();
                let elapsed = now_clone.lock().take().unwrap().elapsed();
                crate::prometheus::JOB_RUN_TIME.observe(elapsed.as_secs_f64());
                Ok(())
            })
            .catch(async move {
                job4.lock().await.catch().await;
                crate::prometheus::JOBS_COMPLETED_FAILED.inc();
                crate::prometheus::JOBS_RUNNING.dec();
                let elapsed = now_clone2.lock().take().unwrap().elapsed();
                crate::prometheus::JOB_RUN_TIME.observe(elapsed.as_secs_f64());
            })
            .build()
    }
}

impl<T: ExecutableJob> MetricizedJob for T {}
