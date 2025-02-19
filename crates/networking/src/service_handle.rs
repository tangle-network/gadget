use crate::{
    blueprint_protocol::InstanceMessageRequest,
    discovery::{PeerInfo, PeerManager},
    service::NetworkMessage,
    types::ProtocolMessage,
};
use crossbeam_channel::{self, Receiver, Sender};
use libp2p::{Multiaddr, PeerId};
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{debug, info};

/// Handle for sending outgoing messages to the network
#[derive(Clone)]
pub struct NetworkSender {
    network_message_sender: Sender<NetworkMessage>,
}

impl NetworkSender {
    #[must_use]
    pub fn new(network_message_sender: Sender<NetworkMessage>) -> Self {
        Self {
            network_message_sender,
        }
    }

    /// Send a protocol message over the network
    pub fn send_message(&self, message: NetworkMessage) -> Result<(), String> {
        self.network_message_sender
            .send(message)
            .map_err(|e| e.to_string())
    }
}

/// Handle for receiving incoming messages from the network
pub struct NetworkReceiver {
    protocol_message_receiver: Receiver<ProtocolMessage>,
}

impl NetworkReceiver {
    #[must_use]
    pub fn new(protocol_message_receiver: Receiver<ProtocolMessage>) -> Self {
        Self {
            protocol_message_receiver,
        }
    }

    /// Get the next protocol message
    pub fn try_recv(&self) -> Result<ProtocolMessage, crossbeam_channel::TryRecvError> {
        self.protocol_message_receiver.try_recv()
    }
}

/// Combined handle for the network service
pub struct NetworkServiceHandle {
    pub local_peer_id: PeerId,
    pub sender: NetworkSender,
    pub receiver: NetworkReceiver,
    pub peer_manager: Arc<PeerManager>,
}

impl Clone for NetworkServiceHandle {
    fn clone(&self) -> Self {
        Self {
            local_peer_id: self.local_peer_id,
            sender: self.sender.clone(),
            receiver: NetworkReceiver::new(self.receiver.protocol_message_receiver.clone()),
            peer_manager: self.peer_manager.clone(),
        }
    }
}

impl NetworkServiceHandle {
    #[must_use]
    pub fn new(
        local_peer_id: PeerId,
        peer_manager: Arc<PeerManager>,
        network_message_sender: Sender<NetworkMessage>,
        protocol_message_receiver: Receiver<ProtocolMessage>,
    ) -> Self {
        Self {
            local_peer_id,
            sender: NetworkSender::new(network_message_sender),
            receiver: NetworkReceiver::new(protocol_message_receiver),
            peer_manager,
        }
    }

    pub fn next_protocol_message(&mut self) -> Option<ProtocolMessage> {
        self.receiver.try_recv().ok()
    }

    #[must_use]
    pub fn peers(&self) -> Vec<PeerId> {
        self.peer_manager
            .get_peers()
            .clone()
            .into_read_only()
            .iter()
            .map(|(peer_id, _)| *peer_id)
            .collect()
    }

    #[must_use]
    pub fn peer_info(&self, peer_id: &PeerId) -> Option<PeerInfo> {
        self.peer_manager.get_peer_info(peer_id)
    }

    #[must_use]
    pub fn send_protocol_message(&self, message: ProtocolMessage) -> Result<(), String> {
        let raw_payload = bincode::serialize(&message).map_err(|err| err.to_string())?;
        if message.routing.recipient.is_some() {
            let instance_message_request = InstanceMessageRequest::Protocol {
                protocol: message.protocol,
                payload: raw_payload,
                metadata: None,
            };
            let recipient = message.routing.recipient.unwrap();
            if let Some(public_key) = recipient.public_key {
                if let Some(peer_id) = self.peer_manager.get_peer_id_from_public_key(&public_key) {
                    self.send_network_message(NetworkMessage::InstanceRequest {
                        peer: peer_id,
                        request: instance_message_request,
                    })?;
                    debug!("Sent outbound p2p `NetworkMessage` to {:?}", peer_id);
                }
            }
        } else {
            let gossip_message = NetworkMessage::GossipMessage {
                source: self.local_peer_id,
                topic: message.protocol,
                message: raw_payload,
            };
            self.send_network_message(gossip_message)?;
            debug!("Sent outbound gossip `NetworkMessage`");
        }

        Ok(())
    }

    pub(crate) fn send_network_message(&self, message: NetworkMessage) -> Result<(), String> {
        self.sender.send_message(message)
    }

    #[must_use]
    pub fn get_listen_addr(&self) -> Option<Multiaddr> {
        // Get the first peer info for our local peer ID
        if let Some(peer_info) = self.peer_manager.get_peer_info(&self.local_peer_id) {
            // Return the first address from our peer info
            peer_info.addresses.iter().next().cloned()
        } else {
            None
        }
    }

    /// Split the handle into separate sender and receiver
    #[must_use]
    pub fn split(self) -> (NetworkSender, NetworkReceiver) {
        (self.sender, self.receiver)
    }
}

/// We might also bundle a `JoinHandle` so the user can await its completion if needed.
pub struct NetworkServiceTaskHandle {
    /// The join handle for the background service task.
    pub service_task: JoinHandle<()>,
}
