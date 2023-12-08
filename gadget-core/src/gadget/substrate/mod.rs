use crate::gadget::manager::AbstractGadget;
use async_trait::async_trait;
use auto_impl::auto_impl;
use sc_client_api::{BlockImportNotification, FinalityNotification};
use sp_runtime::traits::Block;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;

pub struct SubstrateGadget<Module: SubstrateGadgetModule> {
    module: Module,
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
    type Client: Client<Self::Block>;

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

#[async_trait]
#[auto_impl(Arc)]
pub trait Client<B: Block>: Send + Sync {
    async fn get_next_finality_notification(&self) -> Option<FinalityNotification<B>>;
    async fn get_latest_finality_notification(&self) -> Option<FinalityNotification<B>>;
    async fn get_next_block_import_notification(&self) -> Option<BlockImportNotification<B>>;
}

impl<Module> SubstrateGadget<Module>
where
    Module: SubstrateGadgetModule,
{
    pub fn new(client: Module::Client, module: Module) -> Self {
        Self {
            module,
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
        self.client.get_next_finality_notification().await
    }

    async fn get_next_block_import_notification(&self) -> Option<Self::BlockImportNotification> {
        self.client.get_next_block_import_notification().await
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
