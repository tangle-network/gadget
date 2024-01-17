use crate::debug_logger::DebugLogger;
use crate::gadget::message::GadgetProtocolMessage;
use crate::gadget::work_manager::WorkManager;
use crate::protocol::AsyncProtocolRemote;
use crate::Error;
use async_trait::async_trait;
use gadget_core::gadget::substrate::{Client, SubstrateGadgetModule};
use gadget_core::job::BuiltExecutableJobWrapper;
use gadget_core::job_manager::{PollMethod, ProtocolWorkManager};
use network::Network;
use parking_lot::RwLock;
use sc_client_api::{BlockImportNotification, FinalityNotification};
use sp_runtime::traits::{Block, Header};
use sp_runtime::SaturatedConversion;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

pub mod message;
pub mod network;
pub mod work_manager;

/// Used as a module to place inside the SubstrateGadget
pub struct Module<B, C, N, P> {
    protocol: P,
    network: N,
    job_manager: ProtocolWorkManager<WorkManager>,
    clock: Arc<RwLock<Option<u64>>>,
    _pd: PhantomData<(B, C)>,
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

impl<C: Client<B>, B: Block, N: Network, P: TangleGadgetProtocol<B>> Module<B, C, N, P> {
    pub fn new(network: N, protocol: P, job_manager: ProtocolWorkManager<WorkManager>) -> Self {
        let clock = job_manager.utility.clock.clone();
        Module {
            protocol,
            job_manager,
            network,
            clock,
            _pd: Default::default(),
        }
    }
}

#[async_trait]
impl<C: Client<B>, B: Block, N: Network, M: TangleGadgetProtocol<B>> SubstrateGadgetModule
    for Module<B, C, N, M>
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
        let now: u64 = (*notification.header.number()).saturated_into();
        *self.clock.write() = Some(now);

        if let Some(jobs) = self
            .protocol
            .get_next_jobs(&notification, now, &self.job_manager)
            .await?
        {
            for job in jobs {
                let (remote, protocol) = job;
                let task_id = remote.associated_task_id;
                if self.job_manager.job_exists(&task_id) {
                    // log::warn!(target: "gadget", "The job requested for initialization is already running or enqueued, skipping submission");
                    continue;
                }

                if let Err(err) =
                    self.job_manager
                        .push_task(task_id, false, Arc::new(remote), protocol)
                {
                    log::error!(target: "gadget", "Failed to push task to job manager: {err:?}");
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
pub trait TangleGadgetProtocol<B: Block>: Send + Sync {
    async fn get_next_jobs(
        &self,
        notification: &FinalityNotification<B>,
        now: u64,
        job_manager: &ProtocolWorkManager<WorkManager>,
    ) -> Result<Option<Vec<Job>>, Error>;
    async fn process_block_import_notification(
        &self,
        notification: BlockImportNotification<B>,
        job_manager: &ProtocolWorkManager<WorkManager>,
    ) -> Result<(), Error>;
    async fn process_error(&self, error: Error, job_manager: &ProtocolWorkManager<WorkManager>);
    fn logger(&self) -> &DebugLogger;
    fn get_work_manager_config(&self) -> WorkManagerConfig {
        WorkManagerConfig {
            interval: DEFAULT_POLL_INTERVAL,
            max_active_tasks: DEFAULT_MAX_ACTIVE_TASKS,
            max_pending_tasks: DEFAULT_MAX_PENDING_TASKS,
        }
    }
}
