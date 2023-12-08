use crate::util::DebugLogger;
use crate::MpEcdsaProtocolConfig;
use async_trait::async_trait;
use gadget_core::job_manager::WorkManagerInterface;
use std::error::Error;
use webb_gadget::gadget::message::GadgetProtocolMessage;
use webb_gadget::gadget::network::Network;
use webb_gadget::gadget::work_manager::WebbWorkManager;

#[derive(Clone)]
pub struct GossipNetwork {
    tx_to_network: tokio::sync::mpsc::UnboundedSender<GadgetProtocolMessage>,
}

pub async fn create_network(
    debug_logger: DebugLogger,
    config: &MpEcdsaProtocolConfig,
) -> Result<GossipNetwork, Box<dyn Error>> {
    let (tx_to_network, rx_from_gadget) = tokio::sync::mpsc::unbounded_channel();
    let gossip_network = GossipNetwork { tx_to_network };
    Ok(gossip_network)
}

#[async_trait]
impl Network for GossipNetwork {
    async fn next_message(
        &self,
    ) -> Option<<WebbWorkManager as WorkManagerInterface>::ProtocolMessage> {
        todo!()
    }

    async fn send_message(
        &self,
        message: <WebbWorkManager as WorkManagerInterface>::ProtocolMessage,
    ) -> Result<(), webb_gadget::Error> {
        self.tx_to_network
            .send(message)
            .map_err(|err| webb_gadget::Error::ClientError {
                err: err.to_string(),
            })
    }

    async fn run(&self) -> Result<(), webb_gadget::Error> {
        todo!()
    }
}
