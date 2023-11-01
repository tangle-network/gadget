use crate::gadget::manager::AbstractGadget;
use async_trait::async_trait;
use futures::stream::StreamExt;
use sc_client_api::{
    Backend, BlockImportNotification, BlockchainEvents, FinalityNotification,
    FinalityNotifications, HeaderBackend, ImportNotifications,
};
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct SubstrateGadget<Module: SubstrateGadgetModule> {
    module: Module,
    finality_notification_stream: Mutex<FinalityNotifications<Module::Block>>,
    block_import_notification_stream: Mutex<ImportNotifications<Module::Block>>,
    client: Arc<Module::Client>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct SubstrateGadgetError {}

/// Designed to plug-in to the substrate gadget
#[async_trait]
pub trait SubstrateGadgetModule: Send + Sync {
    type Error: Error + Send;
    type ProtocolMessage: Send;
    type Block: Block;
    type Backend: Backend<Self::Block>;
    type Client: Client<Self::Block, Self::Backend>;

    async fn get_next_protocol_message(&self) -> Option<Self::ProtocolMessage>;
    async fn process_finality_notification(
        &self,
        notification: FinalityNotification<Self::Block>,
    ) -> Result<(), Self::Error>;
    async fn process_block_import_notification(
        &self,
        notification: BlockImportNotification<Self::Block>,
    ) -> Result<(), Self::Error>;
    async fn process_protocol_message(
        &self,
        message: Self::ProtocolMessage,
    ) -> Result<(), Self::Error>;
    async fn process_error(&self, error: Self::Error);
}

impl Display for SubstrateGadgetError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl Error for SubstrateGadgetError {}

pub trait Client<B, BE>:
    BlockchainEvents<B> + HeaderBackend<B> + ProvideRuntimeApi<B> + Send
where
    B: Block,
    BE: Backend<B>,
{
}

impl<Module> SubstrateGadget<Module>
where
    Module: SubstrateGadgetModule,
{
    pub fn new(client: Module::Client, module: Module) -> Self {
        let finality_notification_stream = client.finality_notification_stream();
        let block_import_notification_stream = client.import_notification_stream();

        Self {
            module,
            finality_notification_stream: Mutex::new(finality_notification_stream),
            block_import_notification_stream: Mutex::new(block_import_notification_stream),
            client: Arc::new(client),
        }
    }

    pub fn client(&self) -> &Arc<Module::Client> {
        &self.client
    }
}

#[async_trait]
impl<Module> AbstractGadget for SubstrateGadget<Module>
where
    Module: SubstrateGadgetModule,
{
    type FinalityNotification = FinalityNotification<Module::Block>;
    type BlockImportNotification = BlockImportNotification<Module::Block>;
    type ProtocolMessage = Module::ProtocolMessage;
    type Error = Module::Error;

    async fn get_next_finality_notification(&self) -> Option<Self::FinalityNotification> {
        self.finality_notification_stream.lock().await.next().await
    }

    async fn get_next_block_import_notification(&self) -> Option<Self::BlockImportNotification> {
        self.block_import_notification_stream
            .lock()
            .await
            .next()
            .await
    }

    async fn get_next_protocol_message(&self) -> Option<Self::ProtocolMessage> {
        self.module.get_next_protocol_message().await
    }

    async fn process_finality_notification(
        &self,
        notification: Self::FinalityNotification,
    ) -> Result<(), Self::Error> {
        self.module
            .process_finality_notification(notification)
            .await
    }

    async fn process_block_import_notification(
        &self,
        notification: Self::BlockImportNotification,
    ) -> Result<(), Self::Error> {
        self.module
            .process_block_import_notification(notification)
            .await
    }

    async fn process_protocol_message(
        &self,
        message: Self::ProtocolMessage,
    ) -> Result<(), Self::Error> {
        self.module.process_protocol_message(message).await
    }

    async fn process_error(&self, error: Self::Error) {
        self.module.process_error(error).await
    }
}
