use crate::gadget::message::GadgetProtocolMessage;
use crate::gadget::network::Network;
use crate::gadget::work_manager::WebbWorkManager;
use crate::protocol::AsyncProtocolRemote;
use crate::Error;
use async_trait::async_trait;
use gadget_core::gadget::substrate::{Client, SubstrateGadgetModule};
use gadget_core::job::BuiltExecutableJobWrapper;
use gadget_core::job_manager::{PollMethod, ProtocolWorkManager};
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
pub struct WebbModule<B, C, N, M> {
    protocol: M,
    network: N,
    job_manager: ProtocolWorkManager<WebbWorkManager>,
    clock: Arc<RwLock<Option<u64>>>,
    _pd: PhantomData<(B, C)>,
}

pub const DEFAULT_MAX_ACTIVE_TASKS: usize = 4;
pub const DEFAULT_MAX_PENDING_TASKS: usize = 4;
pub const DEFAULT_POLL_INTERVAL: Option<Duration> = Some(Duration::from_millis(200));

#[derive(Debug)]
pub struct WorkManagerConfig {
    pub interval: Option<Duration>,
    pub max_active_tasks: usize,
    pub max_pending_tasks: usize,
}

impl Default for WorkManagerConfig {
    fn default() -> Self {
        Self {
            interval: DEFAULT_POLL_INTERVAL,
            max_active_tasks: DEFAULT_MAX_ACTIVE_TASKS,
            max_pending_tasks: DEFAULT_MAX_PENDING_TASKS,
        }
    }
}

impl WorkManagerConfig {
    pub fn new(
        interval: Option<Duration>,
        max_active_tasks: usize,
        max_pending_tasks: usize,
    ) -> Self {
        Self {
            interval,
            max_active_tasks,
            max_pending_tasks,
        }
    }

    pub fn manual() -> Self {
        Self {
            interval: None,
            max_active_tasks: DEFAULT_MAX_ACTIVE_TASKS,
            max_pending_tasks: DEFAULT_MAX_PENDING_TASKS,
        }
    }
}

impl<C: Client<B>, B: Block, N: Network, M: WebbGadgetProtocol<B>> WebbModule<B, C, N, M> {
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

#[async_trait]
impl<C: Client<B>, B: Block, N: Network, M: WebbGadgetProtocol<B>> SubstrateGadgetModule
    for WebbModule<B, C, N, M>
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

                if let Err(err) =
                    self.job_manager
                        .push_task(task_id, false, Arc::new(remote), protocol)
                {
                    log::error!("Failed to push task to job manager: {err:?}");
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
pub trait WebbGadgetProtocol<B: Block>: Send + Sync {
    async fn get_next_jobs(
        &self,
        notification: &FinalityNotification<B>,
        now: u64,
        job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) -> Result<Option<Vec<Job>>, Error>;
    async fn process_block_import_notification(
        &self,
        notification: BlockImportNotification<B>,
        job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) -> Result<(), Error>;
    async fn process_error(&self, error: Error, job_manager: &ProtocolWorkManager<WebbWorkManager>);
    fn get_work_manager_config(&self) -> WorkManagerConfig;
}
