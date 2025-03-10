use crate::discovery::peers::VerificationIdentifierKey;
use gadget_crypto::KeyType;
use libp2p::{PeerId, gossipsub::IdentTopic};
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
#[serde(bound = "K: KeyType")]
pub struct MessageRouting<K: KeyType> {
    /// Unique identifier for this message
    pub message_id: u64,
    /// The round/sequence number this message belongs to
    pub round_id: u16,
    /// The sender's information
    pub sender: ParticipantInfo<K>,
    /// Optional recipient information for direct messages
    pub recipient: Option<ParticipantInfo<K>>,
}

/// Information about a participant in the network
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "K: KeyType")]
pub struct ParticipantInfo<K: KeyType> {
    /// The participant's unique ID
    pub id: ParticipantId,
    /// The participant's verification ID key (if known)
    pub verification_id_key: Option<VerificationIdentifierKey<K>>,
}

/// A protocol message that can be sent over the network
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "K: KeyType")]
pub struct ProtocolMessage<K: KeyType> {
    /// The protocol name
    pub protocol: String,
    /// Routing information for the message
    pub routing: MessageRouting<K>,
    /// The actual message payload
    pub payload: Vec<u8>,
}

impl Display for ParticipantId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "P{}", self.0)
    }
}

impl<K: KeyType> Display for MessageRouting<K> {
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

impl<K: KeyType> Display for ParticipantInfo<K> {
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

impl<K: KeyType> Display for ProtocolMessage<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} payload_size={}", self.routing, self.payload.len())
    }
}
