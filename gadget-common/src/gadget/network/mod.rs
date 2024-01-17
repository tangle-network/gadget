use crate::gadget::work_manager::WorkManager;
use crate::Error;
use async_trait::async_trait;
use gadget_core::job_manager::WorkManagerInterface;

pub mod gossip;

#[async_trait]
pub trait Network: Send + Sync + Clone + 'static {
    async fn next_message(&self) -> Option<<WorkManager as WorkManagerInterface>::ProtocolMessage>;
    async fn send_message(
        &self,
        message: <WorkManager as WorkManagerInterface>::ProtocolMessage,
    ) -> Result<(), Error>;

    /// If the network implementation requires a custom runtime, this function
    /// should be manually implemented to keep the network alive
    async fn run(&self) -> Result<(), Error> {
        Ok(())
    }
}
