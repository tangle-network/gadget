mod behaviour;
mod handler;
mod utils;

pub use behaviour::{BlueprintProtocolBehaviour, BlueprintProtocolEvent};
use libp2p::PeerId;

use crate::{discovery::peers::VerificationIdentifierKey, key_types::InstanceSignedMsgSignature};
use serde::{Deserialize, Serialize};

/// A message sent to a specific instance or broadcast to all instances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstanceMessageRequest {
    /// Handshake request with authentication
    Handshake {
        /// Public key for authentication
        verification_id_key: VerificationIdentifierKey,
        /// Signature for verification
        signature: InstanceSignedMsgSignature,
        /// Handshake message
        msg: HandshakeMessage,
    },
    /// Protocol-specific message with custom payload
    Protocol {
        /// Protocol identifier (e.g., "consensus/1.0.0", "sync/1.0.0")
        protocol: String,
        /// Protocol-specific message payload
        payload: Vec<u8>,
        /// Optional metadata for the protocol handler
        metadata: Option<Vec<u8>>,
    },
}

/// Response to an instance message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstanceMessageResponse {
    /// Handshake response with authentication
    Handshake {
        /// Public key for authentication
        verification_id_key: VerificationIdentifierKey,
        /// Signature for verification
        signature: InstanceSignedMsgSignature,
        /// Handshake message
        msg: HandshakeMessage,
    },
    /// Success response with optional data
    Success {
        /// Protocol identifier (e.g., "consensus/1.0.0", "sync/1.0.0")
        protocol: String,
        /// Response data specific to the protocol
        data: Option<Vec<u8>>,
    },
    /// Error response with details
    Error {
        /// Error code
        code: u16,
        /// Error message
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeMessage {
    /// Sender [`PeerId`]
    pub sender: PeerId,
    /// A Unix timestamp in milliseconds
    pub timestamp: u128,
}

impl HandshakeMessage {
    /// Maximum age for a handshake message in milliseconds
    pub const MAX_AGE: u128 = 30_000;

    /// Creates a new handshake message
    ///
    /// # Panics
    /// - If the system time is before the Unix epoch
    #[must_use]
    pub fn new(sender: PeerId) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time went backwards")
            .as_millis();
        Self { sender, timestamp }
    }

    /// Checks if the handshake message is expired
    ///
    /// # Arguments
    /// - `max_age`: Maximum age in milliseconds
    ///
    /// # Returns
    /// - `true` if the message is expired, `false` otherwise
    ///
    /// # Panics
    /// - If the system time is before the Unix epoch
    #[must_use]
    pub fn is_expired(&self, max_age: u128) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time went backwards")
            .as_millis();
        now.saturating_sub(self.timestamp) > max_age
    }

    /// Converts the handshake message to a byte array
    #[must_use]
    pub fn to_bytes(&self, other_peer_id: &PeerId) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(&self.sender.to_bytes());
        bytes.extend(other_peer_id.to_bytes());
        bytes.extend(&self.timestamp.to_be_bytes());
        bytes
    }
}
