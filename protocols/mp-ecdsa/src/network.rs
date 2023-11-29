use crate::MpEcdsaProtocolConfig;
use gadget_core::job_manager::WorkManagerInterface;
use std::error::Error;
use async_trait::async_trait;
use webb_gadget::gadget::network::Network;
use webb_gadget::gadget::work_manager::WebbWorkManager;

#[derive(Clone)]
pub struct GossipNetwork;

pub async fn create_network(
    config: &MpEcdsaProtocolConfig,
) -> Result<GossipNetwork, Box<dyn Error>> {
    let gossip_bootnode = config.gossip_bootnode.parse()?;
    let gossip_config = WebbGossipConfig {
        bootnodes: vec![gossip_bootnode],
        ..Default::default()
    };
    let gossip_network = GossipNetwork::new(gossip_config).await?;
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
        todo!()
    }

    async fn run(&self) -> Result<(), webb_gadget::Error> {
        todo!()
    }
}
