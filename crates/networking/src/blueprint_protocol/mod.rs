mod behaviour;
mod handler;

pub use behaviour::{BlueprintProtocolBehaviour, BlueprintProtocolEvent};

use crate::key_types::{InstanceMsgPublicKey, InstanceSignedMsgSignature};
use serde::{Deserialize, Serialize};

/// A message sent to a specific instance or broadcast to all instances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstanceMessageRequest {
    /// Handshake request with authentication
    Handshake {
        /// Public key for authentication
        public_key: InstanceMsgPublicKey,
        /// Signature for verification
        signature: InstanceSignedMsgSignature,
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
        public_key: InstanceMsgPublicKey,
        /// Signature for verification
        signature: InstanceSignedMsgSignature,
    },
    /// Success response with optional data
    Success {
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
    /// Protocol-specific response
    Protocol {
        /// Protocol-specific response data
        data: Vec<u8>,
    },
}
