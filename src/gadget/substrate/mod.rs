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
use tokio::sync::Mutex;

pub struct SubstrateGadget<B: Block, BE, C, API, Module> {
    client: C,
    module: Module,
    finality_notification_stream: Mutex<FinalityNotifications<B>>,
    block_import_notification_stream: Mutex<ImportNotifications<B>>,
    _pd: std::marker::PhantomData<(B, BE, API)>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct SubstrateGadgetError {}

/// Designed to plug-in to the substrate gadget
#[async_trait]
pub trait SubstrateGadgetModule<Gadget: AbstractGadget>: Send + Sync {
    type Error: Error + Send;
    type ProtocolMessage: Send;

    async fn get_next_protocol_message(&self) -> Option<Gadget::ProtocolMessage>;
    async fn process_finality_notification(
        &self,
        notification: Gadget::FinalityNotification,
    ) -> Result<(), Self::Error>;
    async fn process_block_import_notification(
        &self,
        notification: Gadget::BlockImportNotification,
    ) -> Result<(), Self::Error>;
    async fn process_protocol_message(
        &self,
        message: Gadget::ProtocolMessage,
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

impl<B, BE, C, Api, Module> SubstrateGadget<B, BE, C, Api, Module>
where
    B: Block,
    BE: Backend<B>,
    C: Client<B, BE, Api = Api>,
    Module: SubstrateGadgetModule<Self>,
    Api: Send + Sync,
{
    pub fn new(client: C, module: Module) -> Self {
        let finality_notification_stream = client.finality_notification_stream();
        let block_import_notification_stream = client.import_notification_stream();

        Self {
            client,
            module,
            finality_notification_stream: Mutex::new(finality_notification_stream),
            block_import_notification_stream: Mutex::new(block_import_notification_stream),
            _pd: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<B, BE, C, Api, Module> AbstractGadget for SubstrateGadget<B, BE, C, Api, Module>
where
    B: Block,
    BE: Backend<B>,
    C: Client<B, BE, Api = Api>,
    Module: SubstrateGadgetModule<Self>,
    Api: Send + Sync,
{
    type FinalityNotification = FinalityNotification<B>;
    type BlockImportNotification = BlockImportNotification<B>;
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
