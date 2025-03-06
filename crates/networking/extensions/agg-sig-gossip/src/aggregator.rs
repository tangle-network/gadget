use bitvec::prelude::*;
use gadget_networking::types::ParticipantId;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

/// Deterministic aggregator selection based on round and message
pub struct AggregatorSelector {
    // Maximum number of participants
    max_participants: u16,
    // Number of aggregators to select
    num_aggregators: u16,
    // Selected aggregators as a bitvec for efficiency
    selected: BitVec,
    // Current round
    round: u64,
    // Current selection seed
    seed: Vec<u8>,
}

impl AggregatorSelector {
    /// Create a new aggregator selector
    pub fn new(max_participants: u16, num_aggregators: u16) -> Self {
        // Validate inputs
        assert!(
            num_aggregators <= max_participants,
            "Number of aggregators must be <= max participants"
        );
        assert!(num_aggregators > 0, "Must select at least one aggregator");

        Self {
            max_participants,
            num_aggregators,
            selected: bitvec![0; max_participants as usize],
            round: 0,
            seed: Vec::new(),
        }
    }

    /// Select aggregators for a round based on participants
    pub fn select_aggregators(
        &mut self,
        participants: &HashSet<ParticipantId>,
    ) -> AggregatorSelectionResult {
        // Generate seed from participants
        let mut seed = Vec::with_capacity(participants.len() + 8);
        seed.extend_from_slice(
            &participants
                .iter()
                .flat_map(|p| p.0.to_be_bytes())
                .collect::<Vec<_>>(),
        );
        seed.extend_from_slice(&self.round.to_le_bytes());

        // Reset selection bitvec
        self.selected.fill(false);

        // Select aggregators deterministically
        let mut selected_count = 0;
        let mut hash_position = 0;

        while selected_count < self.num_aggregators as usize {
            // Create a deterministic hash based on seed and position
            let hash = self.deterministic_hash(&seed, hash_position);

            // Map hash to a participant index
            let participant_idx = hash % self.max_participants as usize;

            // If this participant is not already selected, select it
            if !self.selected[participant_idx] {
                self.selected.set(participant_idx, true);
                selected_count += 1;
            }

            hash_position += 1;
        }

        // Update state
        self.seed = seed;

        self.build_result()
    }

    /// Check if a participant is selected as an aggregator
    pub fn is_aggregator(&self, participant_id: ParticipantId) -> bool {
        // ParticipantId is just a wrapper around a u16
        let idx = participant_id.0 as usize;
        if idx >= self.selected.len() {
            return false;
        }
        self.selected[idx]
    }

    /// Build the result structure
    fn build_result(&self) -> AggregatorSelectionResult {
        let mut aggregators = HashSet::new();
        for i in 0..self.max_participants as usize {
            if self.selected[i] {
                aggregators.insert(ParticipantId(i as u16));
            }
        }

        AggregatorSelectionResult {
            aggregators,
            seed: self.seed.clone(),
        }
    }

    /// Simple deterministic hash function
    /// In production, use a proper cryptographic hash function
    fn deterministic_hash(&self, data: &[u8], position: usize) -> usize {
        let mut result = 0usize;
        for (i, &byte) in data.iter().enumerate() {
            result = result.wrapping_add(
                (byte as usize).wrapping_mul(i.wrapping_add(position).wrapping_add(1)),
            );
        }
        result
    }
}

/// Result of aggregator selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatorSelectionResult {
    /// Set of selected aggregators
    pub aggregators: HashSet<ParticipantId>,
    /// Selection seed
    pub seed: Vec<u8>,
}

/// Efficient representation of participants using bitvec
#[derive(Clone, Debug)]
pub struct ParticipantSet {
    /// Bit vector representation for quick membership checks
    bitvec: BitVec,
    /// Maximum participant ID
    max_id: u16,
}

impl ParticipantSet {
    /// Create a new participant set
    pub fn new(max_id: u16) -> Self {
        Self {
            bitvec: bitvec![0; max_id as usize + 1],
            max_id,
        }
    }

    /// Add a participant to the set
    pub fn add(&mut self, id: ParticipantId) -> bool {
        if id.0 > self.max_id {
            return false;
        }

        let was_present = self.bitvec[id.0 as usize];
        self.bitvec.set(id.0 as usize, true);
        !was_present
    }

    /// Remove a participant from the set
    pub fn remove(&mut self, id: ParticipantId) -> bool {
        if id.0 > self.max_id {
            return false;
        }

        let was_present = self.bitvec[id.0 as usize];
        self.bitvec.set(id.0 as usize, false);
        was_present
    }

    /// Check if a participant is in the set
    pub fn contains(&self, id: ParticipantId) -> bool {
        if id.0 > self.max_id {
            return false;
        }

        self.bitvec[id.0 as usize]
    }

    /// Get the number of participants in the set
    pub fn len(&self) -> usize {
        self.bitvec.count_ones()
    }

    /// Check if the set is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Convert to a HashSet
    pub fn to_hashset(&self) -> HashSet<ParticipantId> {
        let mut result = HashSet::with_capacity(self.len());
        for i in 0..=self.max_id as usize {
            if self.bitvec[i] {
                result.insert(ParticipantId(i as u16));
            }
        }
        result
    }

    /// Create from a HashSet
    pub fn from_hashset(set: &HashSet<ParticipantId>, max_id: u16) -> Self {
        let mut result = Self::new(max_id);
        for &id in set {
            result.add(id);
        }
        result
    }

    /// Union with another set
    pub fn union(&mut self, other: &Self) {
        assert_eq!(self.max_id, other.max_id, "Sets must have the same max_id");
        self.bitvec |= &other.bitvec;
    }

    /// Intersection with another set
    pub fn intersection(&mut self, other: &Self) {
        assert_eq!(self.max_id, other.max_id, "Sets must have the same max_id");
        self.bitvec &= &other.bitvec;
    }

    /// Difference from another set
    pub fn difference(&mut self, other: &Self) {
        assert_eq!(self.max_id, other.max_id, "Sets must have the same max_id");
        self.bitvec &= !other.bitvec.clone();
    }

    /// Iterate over participants in the set
    pub fn iter(&self) -> impl Iterator<Item = ParticipantId> + '_ {
        self.bitvec.iter_ones().map(|idx| ParticipantId(idx as u16))
    }
}

/// A mapping from participants to data, optimized using bitvec
#[derive(Clone, Debug)]
pub struct ParticipantMap<T> {
    /// Bit vector to track which participants have data
    presence: BitVec,
    /// Storage for participant data
    data: Vec<Option<T>>,
    /// Maximum participant ID
    max_id: u16,
}

impl<T: Clone> ParticipantMap<T> {
    /// Create a new participant map
    pub fn new(max_id: u16) -> Self {
        Self {
            presence: bitvec![0; max_id as usize + 1],
            data: vec![None; max_id as usize + 1],
            max_id,
        }
    }

    /// Insert data for a participant
    pub fn insert(&mut self, id: ParticipantId, value: T) -> Option<T> {
        if id.0 > self.max_id {
            return None;
        }

        let idx = id.0 as usize;
        let old_value = self.data[idx].take();
        self.data[idx] = Some(value);
        self.presence.set(idx, true);
        old_value
    }

    /// Remove data for a participant
    pub fn remove(&mut self, id: ParticipantId) -> Option<T> {
        if id.0 > self.max_id {
            return None;
        }

        let idx = id.0 as usize;
        let old_value = self.data[idx].take();
        self.presence.set(idx, false);
        old_value
    }

    /// Get data for a participant
    pub fn get(&self, id: ParticipantId) -> Option<&T> {
        if id.0 > self.max_id {
            return None;
        }

        let idx = id.0 as usize;
        if self.presence[idx] {
            self.data[idx].as_ref()
        } else {
            None
        }
    }

    /// Get mutable data for a participant
    pub fn get_mut(&mut self, id: ParticipantId) -> Option<&mut T> {
        if id.0 > self.max_id {
            return None;
        }

        let idx = id.0 as usize;
        if self.presence[idx] {
            self.data[idx].as_mut()
        } else {
            None
        }
    }

    /// Check if the map contains data for a participant
    pub fn contains_key(&self, id: ParticipantId) -> bool {
        if id.0 > self.max_id {
            return false;
        }

        self.presence[id.0 as usize]
    }

    /// Convert to a HashMap
    pub fn to_hashmap(&self) -> HashMap<ParticipantId, T> {
        let mut result = HashMap::with_capacity(self.presence.count_ones());
        for i in 0..=self.max_id as usize {
            if self.presence[i] {
                if let Some(value) = &self.data[i] {
                    result.insert(ParticipantId(i as u16), value.clone());
                }
            }
        }
        result
    }

    /// Get the set of participants with data
    pub fn keys(&self) -> ParticipantSet {
        ParticipantSet {
            bitvec: self.presence.clone(),
            max_id: self.max_id,
        }
    }

    /// Get the number of participants with data
    pub fn len(&self) -> usize {
        self.presence.count_ones()
    }

    /// Check if the map is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
