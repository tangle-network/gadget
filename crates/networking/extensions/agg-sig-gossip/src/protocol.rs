use crate::{
    aggregator::{AggregatorSelector, ParticipantMap, ParticipantSet},
    messages::{AggSigMessage, AggregationResult, MaliciousEvidence},
    signature_weight::SignatureWeight,
    zk_proof::{ThresholdProofGenerator, ThresholdWeightProof},
};
use gadget_crypto::aggregation::AggregatableSignature;
use gadget_logging::warn;
use gadget_networking::{
    service_handle::NetworkServiceHandle,
    types::{MessageRouting, ParticipantId, ParticipantInfo, ProtocolMessage},
};
use gadget_std::{
    collections::HashMap,
    collections::HashSet,
    fmt::Debug,
    time::{Duration, Instant},
};
use thiserror::Error;

/// Error types for the aggregation protocol
#[derive(Debug, Error)]
pub enum AggregationError {
    #[error("Invalid signature from participant {0}")]
    InvalidSignature(ParticipantId),

    #[error("Duplicate different signature from participant {0}")]
    DuplicateSignature(ParticipantId),

    #[error("Threshold not met: got {0}, need {1}")]
    ThresholdNotMet(usize, usize),

    #[error("Aggregation error: {0}")]
    AggregationError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] bincode::Error),

    #[error("Key not found")]
    KeyNotFound,

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Timeout")]
    Timeout,

    #[error("Invalid message content")]
    InvalidMessage,
}

/// Configuration for the aggregation protocol
#[derive(Clone)]
pub struct AggregationConfig {
    /// Local participant ID
    pub local_id: ParticipantId,

    /// Maximum number of participants
    pub max_participants: u16,

    /// Number of aggregators to select
    pub num_aggregators: u16,

    /// Timeout for collecting signatures
    pub timeout: Duration,

    /// Protocol ID for message routing
    pub protocol_id: String,
}

impl Default for AggregationConfig {
    fn default() -> Self {
        Self {
            local_id: ParticipantId(0),
            max_participants: 100,
            num_aggregators: 3,
            timeout: Duration::from_secs(5),
            protocol_id: "sig-agg".to_string(),
        }
    }
}

/// State of the aggregation protocol for a single round
#[derive(Clone)]
struct AggregationState<S: AggregatableSignature> {
    /// Messages being signed, with their corresponding aggregates
    /// Map from message to (aggregate signature, contributors)
    messages: HashMap<Vec<u8>, (S::Signature, ParticipantSet)>,

    /// Signatures received from participants, keyed by message and participant ID
    /// Map from message to map of participant IDs to signatures
    signatures_by_message: HashMap<Vec<u8>, ParticipantMap<S::Signature>>,

    /// Set of participants and which messages they've signed
    /// Map from participant ID to set of message hashes they've signed
    participant_messages: HashMap<ParticipantId, HashSet<Vec<u8>>>,

    /// Our own message we're signing (to differentiate from other messages we see)
    local_message: Vec<u8>,

    /// Set of participants identified as malicious
    malicious: ParticipantSet,

    /// Set of participants we've seen signatures from (across all messages)
    seen_signatures: ParticipantSet,

    /// Set of participants we've sent ACKs to
    sent_acks: ParticipantSet,

    /// Start time for timeout tracking
    start_time: Instant,

    /// Flag indicating if the protocol completion has been verified
    protocol_completed: bool,

    /// Verified aggregate signature from a completion message
    verified_completion: Option<(S::Signature, ParticipantSet)>,
}

/// Trait for message verification
pub trait MessageVerifier {
    /// Verify if a message is valid and should be signed
    fn is_valid_message(&self, message: &[u8]) -> bool;
}

/// Default message verifier that accepts all messages
#[derive(Clone)]
pub struct DefaultMessageVerifier;

impl MessageVerifier for DefaultMessageVerifier {
    fn is_valid_message(&self, _message: &[u8]) -> bool {
        true
    }
}

/// Generic signature aggregation protocol
pub struct SignatureAggregationProtocol<S, W, M = DefaultMessageVerifier>
where
    S: AggregatableSignature,
    W: SignatureWeight,
    M: MessageVerifier + Clone,
{
    /// Protocol state for current round
    state: AggregationState<S>,

    /// Protocol configuration
    config: AggregationConfig,

    /// Signature weighting scheme
    weight_scheme: W,

    /// Aggregator selector
    aggregator_selector: AggregatorSelector,

    /// Message verifier
    message_verifier: M,
}

impl<S, W> SignatureAggregationProtocol<S, W, DefaultMessageVerifier>
where
    S: AggregatableSignature,
    W: SignatureWeight,
{
    /// Create a new signature aggregation protocol instance with default message verifier
    pub fn new(config: AggregationConfig, weight_scheme: W) -> Self {
        Self::new_with_verifier(config, weight_scheme, DefaultMessageVerifier)
    }

    pub fn config(&self) -> &AggregationConfig {
        &self.config
    }
}

impl<S, W, M> SignatureAggregationProtocol<S, W, M>
where
    S: AggregatableSignature,
    W: SignatureWeight,
    M: MessageVerifier + Clone,
{
    /// Create a new signature aggregation protocol instance with custom message verifier
    pub fn new_with_verifier(
        config: AggregationConfig,
        weight_scheme: W,
        message_verifier: M,
    ) -> Self {
        // Create default state
        let state = AggregationState {
            messages: HashMap::new(),
            signatures_by_message: HashMap::new(),
            participant_messages: HashMap::new(),
            local_message: Vec::new(),
            malicious: ParticipantSet::new(config.max_participants),
            seen_signatures: ParticipantSet::new(config.max_participants),
            sent_acks: ParticipantSet::new(config.max_participants),
            start_time: Instant::now(),
            protocol_completed: false,
            verified_completion: None,
        };

        // Create aggregator selector
        let aggregator_selector =
            AggregatorSelector::new(config.max_participants, config.num_aggregators);

        Self {
            state,
            config,
            weight_scheme,
            aggregator_selector,
            message_verifier,
        }
    }

    /// Get protocol ID for testing purposes
    pub fn protocol_id(&self) -> &str {
        &self.config.protocol_id
    }

    /// Handle an incoming protocol message
    pub async fn handle_message(
        &mut self,
        protocol_msg: ProtocolMessage<S>,
        public_keys: &HashMap<ParticipantId, S::Public>,
        network_handle: &NetworkServiceHandle<S>,
    ) -> Result<(), AggregationError> {
        let routing = protocol_msg.routing.clone();
        let sender_id = routing.sender.id;

        // Deserialize the message
        let message = bincode::deserialize::<AggSigMessage<S>>(&protocol_msg.payload)?;

        match message {
            AggSigMessage::SignatureShare {
                signature,
                message,
                weight: _,
            } => {
                self.handle_signature_share(
                    sender_id,
                    signature,
                    message,
                    public_keys,
                    network_handle,
                )
                .await
            }
            AggSigMessage::AckSignatures { seen_from, .. } => self.handle_ack_signatures(seen_from),
            AggSigMessage::MaliciousReport { operator, evidence } => {
                self.handle_malicious_report(operator, evidence, public_keys)
            }
            AggSigMessage::ProtocolComplete {
                aggregate_signature,
                message,
                contributors,
            } => self.handle_protocol_complete(
                sender_id,
                aggregate_signature,
                message,
                contributors,
                public_keys,
            ),
        }
    }

    // ----- Handler functions for message types -----

    /// Handle a signature share message
    async fn handle_signature_share(
        &mut self,
        sender_id: ParticipantId,
        signature: S::Signature,
        message: Vec<u8>,
        public_keys: &HashMap<ParticipantId, S::Public>,
        network_handle: &NetworkServiceHandle<S>,
    ) -> Result<(), AggregationError> {
        // Verify the message content is valid
        if !self.message_verifier.is_valid_message(&message) {
            warn!("Invalid message content from {}", sender_id.0);
            return Err(AggregationError::InvalidMessage);
        }

        // Check if we've already seen a signature from this participant for this message
        if self.has_seen_signature_for_message(sender_id, &message) {
            // We already have a signature from this sender for this message, just ignore the new one
            warn!(
                "Duplicate signature attempt from {} for same message, ignoring",
                sender_id.0
            );
            return Ok(());
        }

        // Check if the participant has signed any other messages
        if let Some(previous_messages) = self.state.participant_messages.get(&sender_id).cloned() {
            for prev_msg_hash in previous_messages {
                // Get the original message from the hash
                let has_conflicting_message = self
                    .state
                    .signatures_by_message
                    .keys()
                    .any(|m| self.hash_message(m) == prev_msg_hash && m != &message);

                if has_conflicting_message {
                    // This participant has signed multiple different messages - report as malicious
                    let prev_message = self
                        .state
                        .signatures_by_message
                        .keys()
                        .find(|m| self.hash_message(m) == prev_msg_hash)
                        .cloned()
                        .unwrap_or_default();

                    let prev_sig = self
                        .state
                        .signatures_by_message
                        .get(&prev_message)
                        .and_then(|map| map.get(sender_id))
                        .cloned()
                        .unwrap_or_else(|| signature.clone()); // Fallback if not found

                    warn!("Detected conflicting signatures from {}", sender_id.0);

                    self.mark_participant_malicious(
                        sender_id,
                        MaliciousEvidence::ConflictingSignatures {
                            signature1: prev_sig,
                            signature2: signature.clone(),
                            message1: prev_message,
                            message2: message.clone(),
                        },
                        network_handle,
                    )
                    .await?;

                    return Ok(());
                }
            }
        }

        // Verify the signature
        if !self.verify_signature(sender_id, &signature, &message, public_keys) {
            // Invalid signature - mark as malicious
            warn!("Invalid signature from {}", sender_id.0);
            self.mark_participant_malicious(
                sender_id,
                MaliciousEvidence::InvalidSignature {
                    signature: signature.clone(),
                    message: message.clone(),
                },
                network_handle,
            )
            .await?;
            return Ok(());
        }

        // Store the signature for this message
        self.add_signature(sender_id, signature.clone(), message.clone());

        // Mark this signature as seen
        self.state.seen_signatures.add(sender_id);

        // Track which message this participant signed
        let message_hash = self.hash_message(&message);
        self.state
            .participant_messages
            .entry(sender_id)
            .or_insert_with(HashSet::new)
            .insert(message_hash);

        // Send an acknowledgment
        self.send_ack(sender_id, &message, network_handle).await?;

        // If we're an aggregator, update our aggregate signature for this message
        if self.is_aggregator() {
            self.update_aggregate_for_message(message)?;
        }

        Ok(())
    }

    /// Handle an acknowledgment message
    fn handle_ack_signatures(
        &mut self,
        seen_from: std::collections::HashSet<ParticipantId>,
    ) -> Result<(), AggregationError> {
        // Update our seen signatures set with new information
        let seen_set = ParticipantSet::from_hashset(&seen_from, self.config.max_participants);
        self.state.seen_signatures.union(&seen_set);

        Ok(())
    }

    /// Handle a malicious report message
    fn handle_malicious_report(
        &mut self,
        operator: ParticipantId,
        evidence: MaliciousEvidence<S>,
        public_keys: &HashMap<ParticipantId, S::Public>,
    ) -> Result<(), AggregationError> {
        // Verify the evidence
        let is_malicious = self.verify_malicious_evidence(operator, &evidence, public_keys)?;

        if is_malicious {
            // Add to malicious set
            self.state.malicious.add(operator);

            // Remove from current aggregate if present
            let local_message = self.state.local_message.clone();
            if !local_message.is_empty() {
                if let Some((_, contributors)) = self.state.messages.get_mut(&local_message) {
                    if contributors.contains(operator) {
                        contributors.remove(operator);

                        // We need to rebuild the aggregate signature
                        if self.is_aggregator() {
                            self.rebuild_aggregate()?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle a protocol completion message
    fn handle_protocol_complete(
        &mut self,
        sender_id: ParticipantId,
        aggregate_signature: S::Signature,
        message: Vec<u8>,
        contributors: HashSet<ParticipantId>,
        public_keys: &HashMap<ParticipantId, S::Public>,
    ) -> Result<(), AggregationError> {
        // Skip if protocol is already completed
        if self.state.protocol_completed {
            return Ok(());
        }

        // Verify message matches what we're signing
        if message != self.state.local_message {
            warn!(
                "Completion message has mismatched message from {}",
                sender_id.0
            );
            return Ok(());
        }

        // Convert contributors to our ParticipantSet format
        let contributor_set =
            ParticipantSet::from_hashset(&contributors, self.config.max_participants);

        // Check if weight meets threshold
        let total_weight = self.weight_scheme.calculate_weight(&contributor_set);
        if total_weight < self.weight_scheme.threshold_weight() {
            warn!(
                "Completion message with insufficient weight from {}",
                sender_id.0
            );
            return Ok(());
        }

        // Verify aggregate signature
        let mut public_key_vec = Vec::new();
        let mut missing_keys = false;

        for &id in &contributors {
            if let Some(key) = public_keys.get(&id) {
                public_key_vec.push(key.clone());
            } else {
                missing_keys = true;
                break;
            }
        }

        if missing_keys {
            warn!("Missing public keys for verification from {}", sender_id.0);
            return Ok(());
        }

        // Verify the aggregate signature
        if !S::verify_aggregate(&message, &aggregate_signature, &public_key_vec) {
            warn!(
                "Invalid aggregate signature in completion message from {}",
                sender_id.0
            );
            return Ok(());
        }

        // All checks passed, mark protocol as completed
        self.state.protocol_completed = true;
        self.state.verified_completion = Some((aggregate_signature, contributor_set));

        Ok(())
    }

    // ----- Helper methods for message handlers -----

    /// Mark a participant as malicious and broadcast a report
    async fn mark_participant_malicious(
        &mut self,
        participant_id: ParticipantId,
        evidence: MaliciousEvidence<S>,
        network_handle: &NetworkServiceHandle<S>,
    ) -> Result<(), AggregationError> {
        self.state.malicious.add(participant_id);

        // Create malicious report
        let report_msg = AggSigMessage::MaliciousReport {
            operator: participant_id,
            evidence,
        };

        // Broadcast report
        self.send_message(report_msg, None, network_handle).await
    }

    /// Send an acknowledgment message to a participant
    async fn send_ack(
        &mut self,
        recipient: ParticipantId,
        message: &[u8],
        network_handle: &NetworkServiceHandle<S>,
    ) -> Result<(), AggregationError> {
        let ack_msg = AggSigMessage::AckSignatures {
            message_hash: self.hash_message(message),
            seen_from: self.state.seen_signatures.to_hashset(),
        };

        // Send acknowledgment only to the sender
        self.send_message(ack_msg, Some(recipient), network_handle)
            .await?;
        self.state.sent_acks.add(recipient);

        Ok(())
    }

    /// Verify a signature is valid
    fn verify_signature(
        &self,
        sender_id: ParticipantId,
        signature: &S::Signature,
        message: &[u8],
        public_keys: &HashMap<ParticipantId, S::Public>,
    ) -> bool {
        if let Some(public_key) = public_keys.get(&sender_id) {
            S::verify_single(message, signature, public_key)
        } else {
            warn!("Missing public key for {}", sender_id.0);
            false
        }
    }

    /// Check if we've already seen a signature from a participant for a specific message
    fn has_seen_signature_for_message(
        &self,
        participant_id: ParticipantId,
        message: &[u8],
    ) -> bool {
        self.state
            .signatures_by_message
            .get(message)
            .map(|map| map.contains_key(participant_id))
            .unwrap_or(false)
    }

    /// Verify evidence of malicious behavior
    fn verify_malicious_evidence(
        &self,
        operator: ParticipantId,
        evidence: &MaliciousEvidence<S>,
        public_keys: &HashMap<ParticipantId, S::Public>,
    ) -> Result<bool, AggregationError> {
        match evidence {
            MaliciousEvidence::InvalidSignature { signature, message } => {
                let operator_key = public_keys.get(&operator).ok_or_else(|| {
                    AggregationError::Protocol(format!("Missing public key for {}", operator.0))
                })?;

                Ok(!S::verify_single(message, signature, operator_key))
            }

            MaliciousEvidence::ConflictingSignatures {
                signature1,
                signature2,
                message1,
                message2,
            } => {
                let operator_key = public_keys.get(&operator).ok_or_else(|| {
                    AggregationError::Protocol(format!("Missing public key for {}", operator.0))
                })?;

                // Messages must be different - signing the same message multiple times is allowed
                if message1 == message2 {
                    return Ok(false); // Not malicious to sign the same message multiple times
                }

                // Both signatures must be valid for their respective messages
                Ok(S::verify_single(message1, signature1, operator_key)
                    && S::verify_single(message2, signature2, operator_key))
            }
        }
    }

    /// Update local aggregate with new signature
    fn update_aggregate_for_message(&mut self, message: Vec<u8>) -> Result<(), AggregationError> {
        // Get all signatures for this message
        let sig_map = self
            .state
            .signatures_by_message
            .entry(message.clone())
            .or_insert_with(|| ParticipantMap::new(self.config.max_participants));

        // Create a set of contributors for this message
        let mut contributors = ParticipantSet::new(self.config.max_participants);

        // Iterate through all possible participant IDs
        for id_val in 0..self.config.max_participants {
            let id = ParticipantId(id_val.into());
            if sig_map.contains_key(id) && !self.state.malicious.contains(id) {
                contributors.add(id);
            }
        }

        if contributors.is_empty() {
            return Ok(());
        }

        // Collect signatures from all contributors
        let mut signatures = Vec::new();
        for id in contributors.iter() {
            if let Some(sig) = sig_map.get(id) {
                signatures.push(sig.clone());
            }
        }

        if signatures.is_empty() {
            return Ok(());
        }

        // Aggregate signatures
        let agg_sig = S::aggregate(&signatures).map_err(|e| {
            AggregationError::AggregationError(format!("Failed to aggregate signatures: {:?}", e))
        })?;

        // Store the aggregate
        self.state.messages.insert(message, (agg_sig, contributors));

        Ok(())
    }

    /// Rebuild the aggregate signature after removing malicious operators
    fn rebuild_aggregate(&mut self) -> Result<(), AggregationError> {
        // Get the set of valid contributors (non-malicious)
        let valid_contributors: ParticipantSet = {
            let local_message = self.state.local_message.clone();
            if let Some((_, contributors)) = self.state.messages.get(&local_message) {
                let mut valid = contributors.clone();
                for id in self.state.malicious.iter() {
                    valid.remove(id);
                }
                valid
            } else {
                return Ok(());
            }
        };

        if valid_contributors.is_empty() {
            self.state.messages.remove(&self.state.local_message);
            return Ok(());
        }

        // Collect signatures from valid contributors
        let mut signatures = Vec::new();
        let local_message = self.state.local_message.clone();

        if let Some(sig_map) = self.state.signatures_by_message.get(&local_message) {
            for id in valid_contributors.iter() {
                if let Some(sig) = sig_map.get(id) {
                    signatures.push(sig.clone());
                }
            }
        }

        if signatures.is_empty() {
            self.state.messages.remove(&self.state.local_message);
            return Ok(());
        }

        // Aggregate signatures
        let agg_sig = S::aggregate(&signatures).map_err(|e| {
            AggregationError::AggregationError(format!("Failed to aggregate signatures: {:?}", e))
        })?;

        self.state.messages.insert(
            self.state.local_message.clone(),
            (agg_sig, valid_contributors),
        );
        Ok(())
    }

    /// Hash a message for acknowledgments
    fn hash_message(&self, message: &[u8]) -> Vec<u8> {
        // Simple hash function - in production use a proper cryptographic hash
        let mut hash = Vec::with_capacity(32);
        for chunk in message.chunks(32) {
            let mut byte = 0u8;
            for &b in chunk {
                byte ^= b;
            }
            hash.push(byte);
        }
        // Pad to 32 bytes
        while hash.len() < 32 {
            hash.push(0);
        }
        hash
    }

    /// Check if this node is selected as an aggregator for the current round
    pub fn is_aggregator(&self) -> bool {
        self.aggregator_selector.is_aggregator(self.config.local_id)
    }

    /// Run the protocol until completion or timeout
    pub async fn run(
        &mut self,
        message: Vec<u8>,
        signing_key: &mut S::Secret,
        public_keys: &HashMap<ParticipantId, S::Public>,
        network_handle: &NetworkServiceHandle<S>,
    ) -> Result<(AggregationResult<S>, Option<ThresholdWeightProof>), AggregationError> {
        // Verify the message is valid
        if !self.message_verifier.is_valid_message(&message) {
            return Err(AggregationError::InvalidMessage);
        }

        let participants: HashSet<ParticipantId> = public_keys.keys().cloned().collect();
        self.aggregator_selector.select_aggregators(&participants);

        // Generate our signature
        let local_signature = S::sign_with_secret(signing_key, &message)
            .map_err(|_| AggregationError::KeyNotFound)?;

        // Add our signature
        self.add_signature(
            self.config.local_id,
            local_signature.clone(),
            message.clone(),
        );

        // Update tracking structures
        self.state.seen_signatures.add(self.config.local_id);

        let message_hash = self.hash_message(&message);
        self.state
            .participant_messages
            .entry(self.config.local_id)
            .or_insert_with(HashSet::new)
            .insert(message_hash);

        // If we're an aggregator, initialize our aggregate with our signature
        if self.is_aggregator() {
            self.update_aggregate_for_message(message.clone())?;
        }

        // Broadcast our signature
        let weight = Some(self.weight_scheme.weight(&self.config.local_id));
        let initial_msg = AggSigMessage::SignatureShare {
            signature: local_signature,
            message: message.clone(),
            weight,
        };

        // Send using the network handle
        self.send_message(initial_msg, None, network_handle).await?;

        // Set up a timeout for the overall protocol
        let timeout_deadline = tokio::time::Instant::now() + self.config.timeout;

        // Track if we've sent a completion message
        let mut sent_completion = false;

        // Main protocol loop
        loop {
            // If the protocol is completed (received and verified completion message)
            if self.state.protocol_completed {
                if let Some((signature, contributors)) = &self.state.verified_completion {
                    // Use the verified completion result
                    let total_weight = self.weight_scheme.calculate_weight(contributors);

                    // Create weights map for result
                    let mut weight_map = HashMap::new();
                    for &id in &contributors.to_hashset() {
                        weight_map.insert(id, self.weight_scheme.weight(&id));
                    }

                    let result = AggregationResult {
                        signature: signature.clone(),
                        contributors: contributors.to_hashset(),
                        weights: Some(weight_map),
                        total_weight: Some(total_weight),
                        malicious_participants: self.state.malicious.to_hashset(),
                    };

                    return Ok((result, None)); // No weight proof for verified completion
                }
            }

            // Check if we've reached threshold locally for our message and should send completion message
            if !sent_completion && self.is_aggregator() {
                // Check if our local message has reached threshold
                if let Some((signature, contributors)) =
                    self.state.messages.get(&self.state.local_message)
                {
                    let total_weight = self.weight_scheme.calculate_weight(contributors);

                    if total_weight >= self.weight_scheme.threshold_weight() {
                        // We've reached threshold, broadcast completion message
                        let completion_msg = AggSigMessage::ProtocolComplete {
                            aggregate_signature: signature.clone(),
                            message: self.state.local_message.clone(),
                            contributors: contributors.to_hashset(),
                        };

                        self.send_message(completion_msg, None, network_handle)
                            .await?;
                        sent_completion = true;
                    }
                }
            }

            // Check if we have too many malicious participants to ever reach threshold
            let remaining_weight = self.calculate_remaining_potential_weight();
            if remaining_weight < self.weight_scheme.threshold_weight() {
                // We can't possibly reach the threshold, too many malicious participants
                return Err(AggregationError::ThresholdNotMet(
                    remaining_weight as usize,
                    self.weight_scheme.threshold_weight() as usize,
                ));
            }

            // Check if we've hit the timeout
            if tokio::time::Instant::now() >= timeout_deadline {
                // Timed out, but we still might have reached threshold locally
                return self.build_result();
            }

            // Wait a bit before checking again
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Build the final result of the protocol
    fn build_result(
        &self,
    ) -> Result<(AggregationResult<S>, Option<ThresholdWeightProof>), AggregationError> {
        // Use our local message as the one we're trying to complete
        if let Some((signature, contributors)) = self.state.messages.get(&self.state.local_message)
        {
            let contributors_set = contributors.to_hashset();
            let total_weight = self.weight_scheme.calculate_weight(contributors);

            // Check if we have enough weight
            if total_weight < self.weight_scheme.threshold_weight() {
                return Err(AggregationError::ThresholdNotMet(
                    total_weight as usize,
                    self.weight_scheme.threshold_weight() as usize,
                ));
            }

            // Generate a weight proof if we're an aggregator
            let proof = if self.is_aggregator() {
                // Convert our weight scheme to a map for the proof generator
                let mut weights = HashMap::new();
                for id in &contributors_set {
                    weights.insert(*id, self.weight_scheme.weight(id));
                }

                let proof_gen =
                    ThresholdProofGenerator::new(weights, self.weight_scheme.threshold_weight());

                match proof_gen.generate_proof(&self.state.local_message, &contributors_set) {
                    Ok(proof) => Some(proof),
                    Err(_) => None,
                }
            } else {
                None
            };

            // Create weights map for result
            let mut weight_map = HashMap::new();
            for &id in &contributors_set {
                weight_map.insert(id, self.weight_scheme.weight(&id));
            }

            let result = AggregationResult {
                signature: signature.clone(),
                contributors: contributors_set,
                weights: Some(weight_map),
                total_weight: Some(total_weight),
                malicious_participants: self.state.malicious.to_hashset(),
            };

            Ok((result, proof))
        } else {
            Err(AggregationError::ThresholdNotMet(
                0,
                self.weight_scheme.threshold_weight() as usize,
            ))
        }
    }

    /// Calculate the maximum potential weight we could still reach
    /// This is the current weight plus the weight of all participants we haven't heard from yet
    /// who are not marked as malicious
    fn calculate_remaining_potential_weight(&self) -> u64 {
        // Start with current weight for our local message
        let current_weight =
            if let Some((_, contributors)) = self.state.messages.get(&self.state.local_message) {
                self.weight_scheme.calculate_weight(contributors)
            } else {
                0
            };

        // Add weight of all potential contributors we haven't seen yet
        let mut potential_weight = current_weight;
        let all_participants = (0..self.config.max_participants).map(ParticipantId);

        for participant_id in all_participants {
            // Skip if we've already counted them or they're malicious
            if self.state.seen_signatures.contains(participant_id)
                || self.state.malicious.contains(participant_id)
            {
                continue;
            }

            // Add their potential weight
            potential_weight += self.weight_scheme.weight(&participant_id);
        }

        potential_weight
    }

    /// New helper method to add a signature to our state
    fn add_signature(
        &mut self,
        participant_id: ParticipantId,
        signature: S::Signature,
        message: Vec<u8>,
    ) {
        // Create signature map for this message if it doesn't exist
        let sig_map = self
            .state
            .signatures_by_message
            .entry(message)
            .or_insert_with(|| ParticipantMap::new(self.config.max_participants));

        // Add the signature
        sig_map.insert(participant_id, signature);
    }

    /// Helper to send a protocol message
    async fn send_message(
        &self,
        message: AggSigMessage<S>,
        specific_recipient: Option<ParticipantId>,
        network_handle: &NetworkServiceHandle<S>,
    ) -> Result<(), AggregationError> {
        let payload = bincode::serialize(&message)?;

        let recipient = specific_recipient.map(|id| ParticipantInfo {
            id,
            verification_id_key: None, // This would be filled in by the network layer
        });

        let routing = MessageRouting {
            message_id: 0,
            round_id: 0,
            sender: ParticipantInfo {
                id: self.config.local_id,
                verification_id_key: None, // This would be filled in by the network layer
            },
            recipient,
        };

        // Send the message using the NetworkServiceHandle
        network_handle
            .send(routing, payload)
            .map_err(|e| AggregationError::NetworkError(format!("Failed to send message: {}", e)))
    }
}
