use std::collections::HashSet;
use std::hash::Hash;
use std::marker::PhantomData;

use gadget_networking::types::ParticipantId;

use crate::ParticipantSet;

/// Trait for weighting of participants in signature aggregation
pub trait SignatureWeight {
    /// Returns the weight for a participant
    fn weight(&self, participant_id: &ParticipantId) -> u64;

    /// Returns the total weight of all participants
    fn total_weight(&self) -> u64;

    /// Returns the threshold weight required for a valid aggregate
    fn threshold_weight(&self) -> u64;

    /// Calculates the total weight of a set of participants
    fn calculate_weight(&self, participants: &ParticipantSet) -> u64 {
        participants.iter().map(|id| self.weight(&id)).sum()
    }

    /// Checks if a set of participants meets the required threshold
    fn meets_threshold(&self, participants: &ParticipantSet) -> bool {
        self.calculate_weight(participants) >= self.threshold_weight()
    }
}

/// A simple equal-weight implementation
pub struct EqualWeight {
    total_participants: usize,
    threshold_percentage: u8,
}

impl EqualWeight {
    pub fn new(total_participants: usize, threshold_percentage: u8) -> Self {
        assert!(
            threshold_percentage <= 100,
            "Threshold percentage must be <= 100"
        );
        Self {
            total_participants,
            threshold_percentage,
        }
    }
}

impl SignatureWeight for EqualWeight {
    fn weight(&self, _participant_id: &ParticipantId) -> u64 {
        1
    }

    fn total_weight(&self) -> u64 {
        self.total_participants as u64
    }

    fn threshold_weight(&self) -> u64 {
        (self.total_participants as u64 * self.threshold_percentage as u64) / 100
    }
}

/// A custom weight map implementation
pub struct CustomWeight {
    weights: std::collections::HashMap<ParticipantId, u64>,
    threshold_weight: u64,
}

impl CustomWeight {
    pub fn new(
        weights: std::collections::HashMap<ParticipantId, u64>,
        threshold_weight: u64,
    ) -> Self {
        Self {
            weights,
            threshold_weight,
        }
    }
}

impl SignatureWeight for CustomWeight {
    fn weight(&self, participant_id: &ParticipantId) -> u64 {
        *self.weights.get(participant_id).unwrap_or(&0)
    }

    fn total_weight(&self) -> u64 {
        self.weights.values().sum()
    }

    fn threshold_weight(&self) -> u64 {
        self.threshold_weight
    }
}
