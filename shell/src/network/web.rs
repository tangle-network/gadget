use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use matchbox_socket::PeerId;
use serde::{Deserialize, Serialize};
use sp_core::ecdsa;
use gadget_common::prelude::*;
use gadget_io::tokio::sync::RwLock;
use gadget_io::tokio::sync::Mutex;


#[derive(Clone)]
pub struct MatchboxHandle {
    pub network: &'static str,
    pub connected_peers: Arc<AtomicU32>,
    pub tx_to_outbound: gadget_io::tokio::sync::mpsc::UnboundedSender<IntraNodeWebPayload>,
    pub rx_from_inbound: Arc<Mutex<UnboundedReceiver<Vec<u8>>>>,
    pub ecdsa_peer_id_to_matchbox_id: Arc<RwLock<HashMap<ecdsa::Public, matchbox_socket::PeerId>>>,
}

impl MatchboxHandle {
    pub fn connected_peers(&self) -> usize {
        self.connected_peers
            .load(std::sync::atomic::Ordering::Relaxed) as usize
    }

    pub fn topic(&self) -> &str {
        self.network.clone()
    }
}

pub struct IntraNodeWebPayload {
    topic: String,
    payload: MatchboxGossipOrRequestResponse,
    message_type: MessageType,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum MatchboxGossipOrRequestResponse {
    Gossip(MatchboxMessage),
    Request(MyBehaviourRequest),
    Response(MyBehaviourResponse),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MatchboxMessage {
    pub topic: String,
    pub raw_payload: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum MyBehaviourRequest {
    Handshake {
        ecdsa_public_key: ecdsa::Public,
        signature: ecdsa::Signature,
    },
    Message {
        topic: String,
        raw_payload: Vec<u8>,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum MyBehaviourResponse {
    Handshaked {
        ecdsa_public_key: ecdsa::Public,
        signature: ecdsa::Signature,
    },
    MessageHandled,
}

enum MessageType {
    Broadcast,
    P2P(PeerId),
}

#[async_trait]
impl Network for MatchboxHandle {
    async fn next_message(&self) -> Option<<WorkManager as WorkManagerInterface>::ProtocolMessage> {
        let mut lock = self
            .rx_from_inbound
            .try_lock()
            .expect("There should be only a single caller for `next_message`");

        let message = lock.recv().await?;
        match bincode::deserialize(&message) {
            Ok(message) => Some(message),
            Err(e) => {
                // self.logger
                //     .error(format!("Failed to deserialize message: {e}"));
                drop(lock);
                self.next_message().await
            }
        }
    }

    async fn send_message(
        &self,
        message: <WorkManager as WorkManagerInterface>::ProtocolMessage,
    ) -> Result<(), gadget_common::Error> {
        let message_type = if let Some(to) = message.to_network_id {
            let matchbox_id = self
                .ecdsa_peer_id_to_matchbox_id
                .read()
                .await
                .get(&to)
                .cloned()
                .ok_or_else(|| gadget_common::Error::NetworkError {
                    err: format!(
                        "No Matchbox ID found for ecdsa public key: {to:?}. No handshake happened?"
                    ),
                })?;

            MessageType::P2P(matchbox_id)
        } else {
            MessageType::Broadcast
        };

        let payload_inner = match message_type {
            MessageType::Broadcast => MatchboxGossipOrRequestResponse::Gossip(MatchboxMessage {
                topic: self.topic().to_string(),
                raw_payload: bincode::serialize(&message).expect("Should serialize"),
            }),
            MessageType::P2P(_) => {
                MatchboxGossipOrRequestResponse::Request(MyBehaviourRequest::Message {
                    topic: self.topic().to_string(),
                    raw_payload: bincode::serialize(&message).expect("Should serialize"),
                })
            }
        };

        let payload = IntraNodeWebPayload {
            topic: self.topic().to_string().clone(),
            payload: payload_inner,
            message_type,
        };

        self.tx_to_outbound
            .send(payload)
            .map_err(|e| gadget_common::Error::NetworkError {
                err: format!("Failed to send intra-node payload: {e}"),
            })
    }
}