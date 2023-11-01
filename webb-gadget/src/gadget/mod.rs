use crate::gadget::message::GadgetProtocolMessage;
use crate::gadget::network::Network;
use crate::gadget::work_manager::WebbWorkManager;
use crate::Error;
use async_trait::async_trait;
use gadget_core::gadget::substrate::{Client, SubstrateGadgetModule};
use gadget_core::job_manager::{PollMethod, ProtocolWorkManager};
use parking_lot::RwLock;
use sc_client_api::{Backend, BlockImportNotification, FinalityNotification};
use sp_runtime::traits::{Block, Header};
use sp_runtime::SaturatedConversion;
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod message;
pub mod network;
pub mod work_manager;

/// Used as a module to place inside the SubstrateGadget
pub struct WebbGadget<B, C, BE, N, M> {
    #[allow(dead_code)]
    network: N,
    module: M,
    job_manager: ProtocolWorkManager<WebbWorkManager>,
    from_network: Mutex<tokio::sync::mpsc::UnboundedReceiver<GadgetProtocolMessage>>,
    clock: Arc<RwLock<Option<u64>>>,
    _pd: PhantomData<(B, C, BE)>,
}

const MAX_ACTIVE_TASKS: usize = 4;
const MAX_PENDING_TASKS: usize = 4;

impl<C: Client<B, BE>, B: Block, BE: Backend<B>, N: Network, M: WebbGadgetModule<B>>
    WebbGadget<B, C, BE, N, M>
{
    pub fn new(mut network: N, module: M, now: Option<u64>) -> Self {
        let clock = Arc::new(RwLock::new(now));
        let clock_clone = clock.clone();
        let from_registry = network.take_message_receiver().expect("Should exist");

        let job_manager_zk = WebbWorkManager::new(move || *clock_clone.read());

        let job_manager = ProtocolWorkManager::new(
            job_manager_zk,
            MAX_ACTIVE_TASKS,
            MAX_PENDING_TASKS,
            PollMethod::Interval { millis: 200 },
        );

        WebbGadget {
            module,
            network,
            job_manager,
            clock,
            from_network: Mutex::new(from_registry),
            _pd: Default::default(),
        }
    }
}

#[async_trait]
impl<C: Client<B, BE>, B: Block, BE: Backend<B>, N: Network, M: WebbGadgetModule<B>>
    SubstrateGadgetModule for WebbGadget<B, C, BE, N, M>
{
    type Error = Error;
    type ProtocolMessage = GadgetProtocolMessage;
    type Block = B;
    type Backend = BE;
    type Client = C;

    async fn get_next_protocol_message(&self) -> Option<Self::ProtocolMessage> {
        self.from_network.lock().await.recv().await
    }

    async fn process_finality_notification(
        &self,
        notification: FinalityNotification<B>,
    ) -> Result<(), Self::Error> {
        let now: u64 = (*notification.header.number()).saturated_into();
        *self.clock.write() = Some(now);
        self.module
            .process_finality_notification(notification, now, &self.job_manager)
            .await
    }

    async fn process_block_import_notification(
        &self,
        notification: BlockImportNotification<B>,
    ) -> Result<(), Self::Error> {
        self.module
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
        self.module.process_error(error, &self.job_manager).await
    }
}

#[async_trait]
pub trait WebbGadgetModule<B: Block>: Send + Sync {
    async fn process_finality_notification(
        &self,
        notification: FinalityNotification<B>,
        now: u64,
        job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) -> Result<(), Error>;
    async fn process_block_import_notification(
        &self,
        notification: BlockImportNotification<B>,
        job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) -> Result<(), Error>;
    async fn process_error(&self, error: Error, job_manager: &ProtocolWorkManager<WebbWorkManager>);
}
