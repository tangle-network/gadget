use crate::client::{ClientWithApi, JobsClient};
use crate::debug_logger::DebugLogger;
use crate::environments::GadgetEnvironment;
use crate::gadget::tangle::JobInitMetadata;
use crate::gadget::work_manager::TangleWorkManager;
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

pub mod core;
pub mod message;
pub mod metrics;
pub mod network;
pub mod tangle;
pub mod work_manager;

/// Used as a module to place inside the SubstrateGadget
pub struct GeneralModule<C, N, M, Env: GadgetEnvironment> {
    protocol: M,
    network: N,
    job_manager: ProtocolWorkManager<TangleWorkManager>,
    clock: Arc<RwLock<Option<Env::Clock>>>,
    event_handler: Box<dyn EventHandler<C, N, M, Env::Event, Env::Error>>,
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
        Env: GadgetEnvironment,
        C: ClientWithApi<Env::Event> + ClientWithApi<<Env as GadgetEnvironment>::Client>,
        N: Network<Env::ProtocolMessage, Env::Event>,
        M: GadgetProtocol<Env, C>,
        Env: GadgetEnvironment<Clock = Arc<parking_lot::lock_api::RwLock<parking_lot::RawRwLock, std::option::Option<Env::Clock>>>>,
    > GeneralModule<C, N, M, Env>
{
    pub fn new<Evt: EventHandler<C, N, M, Env::Event, Env::Error>>(
        network: N,
        module: M,
        job_manager: ProtocolWorkManager<Env::JobManager>,
        event_handler: Evt,
    ) -> Self 
    where Evt: EventHandler<C, N, M, <Env as GadgetEnvironment>::Event, <Env as GadgetEnvironment>::Error>,{
        let clock = job_manager.utility.clock().clone();
        GeneralModule {
            protocol: module,
            job_manager,
            event_handler: Box::new(event_handler),
            network,
            clock,
        }
    }
}

#[async_trait]
impl<
        Env: GadgetEnvironment,
        C: ClientWithApi<Env> + ClientWithApi<<Env as GadgetEnvironment>::Client> + gadget_core::gadget::general::Client<<Env as GadgetEnvironment>::Event>,
        N: Network<Env::ProtocolMessage, Env::Event>,
        M: GadgetProtocol<Env, C>,
    > AbstractGadget for GeneralModule<C, N, M, Env>{
    type Event = Env::Event;
    type ProtocolMessage = Env::ProtocolMessage;
    type Error = Env::Error;

    async fn next_event(&self) -> Option<Env::Event> {
        self.protocol.client().client.next_event().await
    }

    async fn get_next_protocol_message(&self) -> Option<Env::ProtocolMessage> {
        self.network.next_message().await
    }

    async fn on_event_received(&self, notification: Self::Event) -> Result<(), Self::Error> {
        self.event_handler.process_event(notification).await
    }

    async fn process_protocol_message(
        &self,
        message: Self::ProtocolMessage,
    ) -> Result<(), Self::Error> {
        self.job_manager
            .deliver_message(message)
            .map(|_| ())
            .map_err(|err| Self::Error::from(format!("{err:?}")))
    }

    async fn process_error(&self, error: Self::Error) {
        self.protocol.process_error(error, &self.job_manager).await
    }
}

#[async_trait]
#[auto_impl::auto_impl(Box)]
pub trait EventHandler<C, N, M, Event, Error>: Send + Sync + 'static {
    async fn process_event(&self, notification: Event) -> Result<(), Error>;
}

// Redirection to the `AbstractGadget` trait, with a placeholder for a client.
#[async_trait]
impl<
        Env: GadgetEnvironment,
        Error: Send + Sync + 'static,
        C: ClientWithApi<Env::Event> + ClientWithApi<<Env as GadgetEnvironment>::Client>,
        N: Network<Env::ProtocolMessage, Env::Event>,
        M: GadgetProtocol<Env, C>,
    > GadgetWithClient<Env::ProtocolMessage, Env::Event, Error> for GeneralModule<C, N, M, Env>
where
    Self: AbstractGadget<Event = Env::Event, Error = Error, ProtocolMessage = Env::ProtocolMessage>,
{
    type Client = C;

    async fn get_next_protocol_message(&self) -> Option<Env::ProtocolMessage> {
        <Self as AbstractGadget>::get_next_protocol_message(self).await
    }

    /// Provided by the developer
    async fn process_event(&self, notification: Env::Event) -> Result<(), Error> {
        <Self as AbstractGadget>::on_event_received(self, notification).await
    }

    async fn process_protocol_message(&self, message: Env::ProtocolMessage) -> Result<(), Error> {
        <Self as AbstractGadget>::process_protocol_message(self, message).await
    }

    async fn process_error(&self, error: Error) {
        <Self as AbstractGadget>::process_error(self, error).await
    }
}

pub type Job<Env> = (AsyncProtocolRemote<Env>, BuiltExecutableJobWrapper);

#[async_trait]
pub trait GadgetProtocol<Env: GadgetEnvironment, C: ClientWithApi<Env::Client>>:
    AsyncProtocol<Env> + Send + Sync + 'static
{
    /// Given an input of a valid and relevant job, return the parameters needed to start the async protocol
    /// Note: the parameters returned must be relevant to the `AsyncProtocol` implementation of this protocol
    ///
    /// In case the participant is not selected for some reason, return an [`Error::ParticipantNotSelected`]
    async fn create_next_job(
        &self,
        job: JobInitMetadata,
        work_manager: &ProtocolWorkManager<Env::JobManager>,
    ) -> Result<<Self as AsyncProtocol<Env>>::AdditionalParams, Error>;

    /// Process an error that may arise from the work manager, async protocol, or the executor
    async fn process_error(
        &self,
        error: Env::Error,
        job_manager: &ProtocolWorkManager<Env::JobManager>,
    );
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
    fn client(&self) -> JobsClient<C, Env::Event>;
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
