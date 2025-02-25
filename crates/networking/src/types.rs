use crate::{discovery::peers::VerificationIdentifierKey, key_types::InstanceMsgPublicKey};
use libp2p::{gossipsub::IdentTopic, PeerId};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// Maximum allowed size for a message payload
pub const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;

/// Unique identifier for a participant in the network
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ParticipantId(pub u16);

impl From<ParticipantId> for u16 {
    fn from(val: ParticipantId) -> Self {
        val.0
    }
}

/// Type of message delivery mechanism
#[derive(Debug, Clone)]
pub enum MessageDelivery {
    /// Broadcast to all peers via gossipsub
    Broadcast {
        /// The topic to broadcast on
        topic: IdentTopic,
    },
    /// Direct P2P message to a specific peer
    Direct {
        /// The target peer ID
        peer_id: PeerId,
    },
}

/// Message routing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRouting {
    /// Unique identifier for this message
    pub message_id: u64,
    /// The round/sequence number this message belongs to
    pub round_id: u16,
    /// The sender's information
    pub sender: ParticipantInfo,
    /// Optional recipient information for direct messages
    pub recipient: Option<ParticipantInfo>,
}

/// Information about a participant in the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantInfo {
    /// The participant's unique ID
    pub id: ParticipantId,
    /// The participant's verification ID key (if known)
    pub verification_id_key: Option<VerificationIdentifierKey>,
}

/// A protocol message that can be sent over the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolMessage {
    /// The protocol name
    pub protocol: String,
    /// Routing information for the message
    pub routing: MessageRouting,
    /// The actual message payload
    pub payload: Vec<u8>,
}

/// Internal representation of a message to be sent
#[derive(Debug)]
pub struct OutboundMessage {
    /// The message to be sent
    pub message: ProtocolMessage,
    /// How the message should be delivered
    pub delivery: MessageDelivery,
}

impl Display for ParticipantId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "P{}", self.0)
    }
}

impl Display for MessageRouting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "msg={} round={} from={} to={:?}",
            self.message_id,
            self.round_id,
            self.sender.id,
            self.recipient.as_ref().map(|r| r.id)
        )
    }
}

impl Display for ParticipantInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} key={}",
            self.id,
            if self.verification_id_key.is_some() {
                "yes"
            } else {
                "no"
            }
        )
    }
}

impl Display for ProtocolMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} payload_size={}", self.routing, self.payload.len())
    }
}
