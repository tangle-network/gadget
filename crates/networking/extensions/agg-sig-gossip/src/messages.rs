use gadget_crypto::aggregation::AggregatableSignature;
use gadget_networking::types::ParticipantId;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;

/// Protocol message types for signature aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "S: AggregatableSignature")]
pub enum AggSigMessage<S: AggregatableSignature> {
    /// Initial signature share from a participant
    SignatureShare {
        /// The signature
        signature: S::Signature,
        /// The message being signed
        message: Vec<u8>,
        /// Weight of the signer (optional for weighted protocols)
        weight: Option<u64>,
    },
    /// ACK message to confirm receipt of signatures
    // This helps optimize gossip by preventing redundant message sends
    AckSignatures {
        /// Message hash being acknowledged
        message_hash: Vec<u8>,
        /// Set of signatures seen from these participants
        seen_from: HashSet<ParticipantId>,
    },

    /// Report malicious behavior
    MaliciousReport {
        /// The accused operator
        operator: ParticipantId,
        /// Evidence of malicious behavior
        evidence: MaliciousEvidence<S>,
    },

    /// Protocol completion message
    /// Sent when a node has enough signatures to meet the threshold
    ProtocolComplete {
        /// The aggregated signature
        aggregate_signature: S::Signature,
        /// Message that was signed
        message: Vec<u8>,
        /// Set of participants who contributed to the signature
        contributors: HashSet<ParticipantId>,
    },
}

/// Evidence of malicious behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "S: AggregatableSignature")]
pub enum MaliciousEvidence<S: AggregatableSignature> {
    /// Invalid signature that doesn't verify
    InvalidSignature {
        /// Signature
        signature: S::Signature,
        /// Message being signed
        message: Vec<u8>,
    },
    /// Conflicting valid signatures for different messages in the same round
    ConflictingSignatures {
        /// First signature
        signature1: S::Signature,
        /// Second signature
        signature2: S::Signature,
        /// First message being signed
        message1: Vec<u8>,
        /// Second message being signed
        message2: Vec<u8>,
    },
}

/// Information about aggregators for the current round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatorInfo {
    /// Set of designated aggregators for the current round
    pub aggregators: HashSet<ParticipantId>,
    /// Selection seed used to determine aggregators
    pub selection_seed: Vec<u8>,
    /// Round ID this selection applies to
    pub round: u64,
}

/// Result of the aggregation protocol
#[derive(Debug, Clone)]
pub struct AggregationResult<S: AggregatableSignature> {
    /// The aggregated signature
    pub signature: S::Signature,
    /// Set of participants who contributed to the signature
    pub contributors: HashSet<ParticipantId>,
    /// Map of participant IDs to their weights (if using weighted aggregation)
    pub weights: Option<HashMap<ParticipantId, u64>>,
    /// Total weight of the aggregate signature
    pub total_weight: Option<u64>,
    /// Set of participants identified as malicious
    pub malicious_participants: HashSet<ParticipantId>,
}
