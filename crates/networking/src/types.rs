use crate::{behaviours::GossipOrRequestResponse, key_types::GossipMsgPublicKey};
use libp2p::{gossipsub::IdentTopic, PeerId};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// Maximum allowed size for a Signed Message.
pub const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;

/// Unique identifier for a party
pub type UserID = u16;

/// Type of message to be sent
pub enum MessageType {
    Broadcast,
    P2P(PeerId),
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub struct IdentifierInfo {
    pub message_id: u64,
    pub round_id: u16,
}

impl Display for IdentifierInfo {
    fn fmt(&self, f: &mut gadget_std::fmt::Formatter<'_>) -> gadget_std::fmt::Result {
        let message_id = format!("message_id: {}", self.message_id);
        let round_id = format!("round_id: {}", self.round_id);
        write!(f, "{} {}", message_id, round_id)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct ParticipantInfo {
    pub user_id: u16,
    pub public_key: Option<GossipMsgPublicKey>,
}

impl Display for ParticipantInfo {
    fn fmt(&self, f: &mut gadget_std::fmt::Formatter<'_>) -> gadget_std::fmt::Result {
        let public_key = self
            .public_key
            .map(|key| format!("public_key: {:?}", key))
            .unwrap_or_default();
        write!(f, "user_id: {}, {}", self.user_id, public_key)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProtocolMessage {
    pub identifier_info: IdentifierInfo,
    pub sender: ParticipantInfo,
    pub recipient: Option<ParticipantInfo>,
    pub payload: Vec<u8>,
}

impl Display for ProtocolMessage {
    fn fmt(&self, f: &mut gadget_std::fmt::Formatter<'_>) -> gadget_std::fmt::Result {
        write!(
            f,
            "identifier_info: {}, sender: {}, recipient: {:?}, payload: {:?}",
            self.identifier_info, self.sender, self.recipient, self.payload
        )
    }
}

pub struct IntraNodePayload {
    pub topic: IdentTopic,
    pub payload: GossipOrRequestResponse,
    pub message_type: MessageType,
}

impl gadget_std::fmt::Debug for IntraNodePayload {
    fn fmt(&self, f: &mut gadget_std::fmt::Formatter<'_>) -> gadget_std::fmt::Result {
        f.debug_struct("IntraNodePayload")
            .field("topic", &self.topic)
            .finish_non_exhaustive()
    }
}
