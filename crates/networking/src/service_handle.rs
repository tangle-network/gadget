use crate::{
    blueprint_protocol::InstanceMessageRequest, key_types::InstanceMsgPublicKey,
    service::NetworkMessage, types::ProtocolMessage,
};
use crossbeam_channel::{self, Receiver, Sender};
use dashmap::DashMap;
use gadget_logging::info;
use libp2p::PeerId;
use std::sync::Arc;
use tokio::task::JoinHandle;

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
    pub public_keys_to_peer_ids: Arc<DashMap<InstanceMsgPublicKey, PeerId>>,
}

impl Clone for NetworkServiceHandle {
    fn clone(&self) -> Self {
        Self {
            local_peer_id: self.local_peer_id,
            sender: self.sender.clone(),
            receiver: NetworkReceiver::new(self.receiver.protocol_message_receiver.clone()),
            public_keys_to_peer_ids: self.public_keys_to_peer_ids.clone(),
        }
    }
}

impl NetworkServiceHandle {
    #[must_use]
    pub fn new(
        local_peer_id: PeerId,
        public_keys_to_peer_ids: Arc<DashMap<InstanceMsgPublicKey, PeerId>>,
        network_message_sender: Sender<NetworkMessage>,
        protocol_message_receiver: Receiver<ProtocolMessage>,
    ) -> Self {
        Self {
            local_peer_id,
            sender: NetworkSender::new(network_message_sender),
            receiver: NetworkReceiver::new(protocol_message_receiver),
            public_keys_to_peer_ids,
        }
    }

    pub async fn next_protocol_message(&mut self) -> Option<ProtocolMessage> {
        self.receiver.try_recv().ok()
    }

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
                if let Some(peer_id) = self.public_keys_to_peer_ids.get(&public_key) {
                    self.sender.send_message(NetworkMessage::InstanceRequest {
                        peer: *peer_id,
                        request: instance_message_request,
                    })?;
                    info!("Sent outbound p2p `NetworkMessage` to {:?}", peer_id);
                }
            }
        } else {
            let gossip_message = NetworkMessage::GossipMessage {
                source: self.local_peer_id,
                topic: message.protocol,
                message: raw_payload,
            };
            self.sender.send_message(gossip_message)?;
            info!("Sent outbound gossip `NetworkMessage`");
        }

        Ok(())
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
