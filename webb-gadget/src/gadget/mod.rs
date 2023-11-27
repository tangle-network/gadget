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

const MAX_ACTIVE_TASKS: usize = 4;
const MAX_PENDING_TASKS: usize = 4;

impl<C: Client<B>, B: Block, N: Network, M: WebbGadgetProtocol<B>> WebbModule<B, C, N, M> {
    pub fn new(network: N, module: M, now: Option<u64>) -> Self {
        let clock = Arc::new(RwLock::new(now));
        let clock_clone = clock.clone();

        let job_manager_zk = WebbWorkManager::new(move || *clock_clone.read());

        let job_manager = ProtocolWorkManager::new(
            job_manager_zk,
            MAX_ACTIVE_TASKS,
            MAX_PENDING_TASKS,
            PollMethod::Interval { millis: 200 },
        );

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

        while let Some(job) = self
            .protocol
            .get_next_job(&notification, now, &self.job_manager)
            .await?
        {
            let (remote, protocol) = job;
            let task_id = remote.associated_task_id;

            if let Err(err) = self
                .job_manager
                .push_task(task_id, false, Arc::new(remote), protocol)
            {
                log::error!("Failed to push task to job manager: {err:?}");
            }
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
    async fn get_next_job(
        &self,
        notification: &FinalityNotification<B>,
        now: u64,
        job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) -> Result<Option<Job>, Error>;
    async fn process_block_import_notification(
        &self,
        notification: BlockImportNotification<B>,
        job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) -> Result<(), Error>;
    async fn process_error(&self, error: Error, job_manager: &ProtocolWorkManager<WebbWorkManager>);
}
