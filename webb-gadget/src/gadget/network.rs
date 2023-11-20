use crate::gadget::work_manager::WebbWorkManager;
use crate::Error;
use async_trait::async_trait;
use gadget_core::job_manager::WorkManagerInterface;

#[async_trait]
pub trait Network: Send + Sync + Clone {
    async fn next_message(
        &self,
    ) -> Option<<WebbWorkManager as WorkManagerInterface>::ProtocolMessage>;
    async fn send_message(
        &self,
        message: <WebbWorkManager as WorkManagerInterface>::ProtocolMessage,
    ) -> Result<(), Error>;
    async fn run(&self) -> Result<(), Error>;
}
