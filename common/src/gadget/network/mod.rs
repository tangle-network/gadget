use crate::environments::GadgetEnvironment;
use crate::Error;
use async_trait::async_trait;
use gadget_core::gadget::manager::AbstractGadget;
use sp_core::ecdsa;

#[async_trait]
pub trait Network<Env: GadgetEnvironment>: Send + Sync + Clone + 'static {
    async fn next_message(&self) -> Option<Env::ProtocolMessage>;
    async fn send_message(&self, message: Env::ProtocolMessage) -> Result<(), Error>;

    /// The ID of the king. Only relevant for ZkSaaS networks
    fn greatest_authority_id(&self) -> Option<ecdsa::Public> {
        None
    }
    /// If the network implementation requires a custom runtime, this function
    /// should be manually implemented to keep the network alive
    async fn run(self) -> Result<(), Error> {
        Ok(())
    }
}
