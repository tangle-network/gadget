mod aggregator;
mod messages;
mod protocol;
mod signature_weight;
mod zk_proof;

// Re-export the main components
pub use aggregator::{AggregatorSelector, ParticipantMap, ParticipantSet};
pub use messages::{AggSigMessage, AggregationResult, MaliciousEvidence};
pub use protocol::{AggregationConfig, AggregationError, SignatureAggregationProtocol};
pub use zk_proof::{
    ThresholdProofGenerator, ThresholdProofVerifier, ThresholdWeightProof, ZkProofError,
};

#[cfg(test)]
pub mod tests;
