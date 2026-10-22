//! Message storage with efficient indexing for gossip protocols
//!
//! This module provides [`MessageStore`] for storing and retrieving messages
//! with multiple index types (by hash, sender, timestamp).

use crate::dedup::MessageHash;
use crate::network::PeerId;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::time::Duration;
use hashbrown::HashMap;
use parking_lot::RwLock;
#[cfg(feature = "std")]
use std::time::Instant;
#[cfg(not(feature = "std"))]
use tokio::time::Instant;

/// Entry in the message store
#[derive(Debug, Clone)]
pub struct MessageEntry<M> {
    /// The message content
    pub message: M,
    /// Hash of the message for deduplication
    pub hash: MessageHash,
    /// Peer that sent this message (if known)
    pub sender: Option<PeerId>,
    /// When the message was received
    pub received_at: Instant,
    /// Whether the message has been processed
    pub processed: bool,
}

impl<M> MessageEntry<M> {
    /// Create a new message entry
    #[must_use]
    pub fn new(message: M, hash: MessageHash, sender: Option<PeerId>) -> Self {
        Self {
            message,
            hash,
            sender,
            received_at: Instant::now(),
            processed: false,
        }
    }

    /// Mark the message as processed
    pub fn mark_processed(&mut self) {
        self.processed = true;
    }

    /// Get the age of this message
    #[must_use]
    pub fn age(&self) -> Duration {
        self.received_at.elapsed()
    }
}

/// Configuration for the message store
#[derive(Debug, Clone)]
pub struct StoreConfig {
    /// Maximum number of messages to store
    pub max_messages: usize,
    /// Whether to index messages by sender
    pub index_by_sender: bool,
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self {
            max_messages: 10_000,
            index_by_sender: true,
        }
    }
}

/// Thread-safe message store with multiple indexes
///
/// Provides efficient access to messages by:
/// - Hash (primary key)
/// - Sender (secondary index)
/// - Insertion order (for iteration and cleanup)
pub struct MessageStore<M> {
    /// Primary storage: hash -> entry
    messages: RwLock<HashMap<MessageHash, MessageEntry<M>>>,
    /// Index: sender -> list of message hashes
    by_sender: RwLock<HashMap<PeerId, Vec<MessageHash>>>,
    /// Ordered by insertion time (for eviction)
    insertion_order: RwLock<BTreeMap<Instant, MessageHash>>,
    /// Configuration
    config: StoreConfig,
}

impl<M: Clone> MessageStore<M> {
    /// Create a new message store with the given configuration
    #[must_use]
    pub fn new(config: StoreConfig) -> Self {
        Self {
            messages: RwLock::new(HashMap::with_capacity(config.max_messages)),
            by_sender: RwLock::new(HashMap::new()),
            insertion_order: RwLock::new(BTreeMap::new()),
            config,
        }
    }

    /// Create a new message store with default configuration
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(StoreConfig::default())
    }

    /// Insert a message into the store
    ///
    /// Returns `true` if the message was inserted (new), `false` if it already existed.
    pub fn insert(&self, entry: MessageEntry<M>) -> bool {
        let hash = entry.hash;
        let sender = entry.sender;
        let received_at = entry.received_at;

        // Check if we need to evict old entries
        self.maybe_evict();

        let mut messages = self.messages.write();

        // Don't insert if already exists
        if messages.contains_key(&hash) {
            return false;
        }

        // Insert into primary storage
        messages.insert(hash, entry);
        drop(messages);

        // Update sender index
        if self.config.index_by_sender {
            if let Some(peer_id) = sender {
                self.by_sender
                    .write()
                    .entry(peer_id)
                    .or_default()
                    .push(hash);
            }
        }

        // Update insertion order
        self.insertion_order.write().insert(received_at, hash);

        true
    }

    /// Get a message by its hash
    #[must_use]
    pub fn get(&self, hash: &MessageHash) -> Option<MessageEntry<M>> {
        self.messages.read().get(hash).cloned()
    }

    /// Get a message by its hash (mutable access for marking processed)
    pub fn get_mut<F, R>(&self, hash: &MessageHash, f: F) -> Option<R>
    where
        F: FnOnce(&mut MessageEntry<M>) -> R,
    {
        self.messages.write().get_mut(hash).map(f)
    }

    /// Check if a message exists by hash
    #[must_use]
    pub fn contains(&self, hash: &MessageHash) -> bool {
        self.messages.read().contains_key(hash)
    }

    /// Get all messages from a specific sender
    #[must_use]
    pub fn get_by_sender(&self, sender: &PeerId) -> Vec<MessageEntry<M>> {
        let by_sender = self.by_sender.read();
        let messages = self.messages.read();

        by_sender
            .get(sender)
            .map(|hashes| {
                hashes
                    .iter()
                    .filter_map(|h| messages.get(h).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all unprocessed messages
    #[must_use]
    pub fn get_unprocessed(&self) -> Vec<MessageEntry<M>> {
        self.messages
            .read()
            .values()
            .filter(|e| !e.processed)
            .cloned()
            .collect()
    }

    /// Mark a message as processed
    ///
    /// Returns `true` if the message existed and was marked.
    pub fn mark_processed(&self, hash: &MessageHash) -> bool {
        self.messages
            .write()
            .get_mut(hash)
            .map(|entry| entry.mark_processed())
            .is_some()
    }

    /// Remove a message by hash
    ///
    /// Returns the removed entry if it existed.
    pub fn remove(&self, hash: &MessageHash) -> Option<MessageEntry<M>> {
        let entry = self.messages.write().remove(hash)?;

        // Remove from sender index
        if let Some(sender) = entry.sender {
            if let Some(hashes) = self.by_sender.write().get_mut(&sender) {
                hashes.retain(|h| h != hash);
            }
        }

        // Remove from insertion order
        self.insertion_order.write().remove(&entry.received_at);

        Some(entry)
    }

    /// Get the number of stored messages
    #[must_use]
    pub fn len(&self) -> usize {
        self.messages.read().len()
    }

    /// Check if the store is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.messages.read().is_empty()
    }

    /// Clear all messages
    pub fn clear(&self) {
        self.messages.write().clear();
        self.by_sender.write().clear();
        self.insertion_order.write().clear();
    }

    /// Evict oldest messages if over capacity
    fn maybe_evict(&self) {
        let current_len = self.messages.read().len();
        if current_len < self.config.max_messages {
            return;
        }

        // Evict 10% of oldest messages
        let to_evict = (self.config.max_messages / 10).max(1);
        let mut insertion_order = self.insertion_order.write();
        let mut messages = self.messages.write();
        let mut by_sender = self.by_sender.write();

        let hashes_to_remove: Vec<_> = insertion_order
            .iter()
            .take(to_evict)
            .map(|(_, hash)| *hash)
            .collect();

        for hash in &hashes_to_remove {
            if let Some(entry) = messages.remove(hash) {
                // Remove from sender index
                if let Some(sender) = entry.sender {
                    if let Some(hashes) = by_sender.get_mut(&sender) {
                        hashes.retain(|h| h != hash);
                    }
                }
            }
        }

        // Remove from insertion order
        let times_to_remove: Vec<_> = insertion_order
            .iter()
            .take(to_evict)
            .map(|(t, _)| *t)
            .collect();
        for time in times_to_remove {
            insertion_order.remove(&time);
        }
    }

    /// Remove all messages older than the given duration
    pub fn remove_older_than(&self, max_age: Duration) {
        let cutoff = Instant::now() - max_age;
        let mut insertion_order = self.insertion_order.write();
        let mut messages = self.messages.write();
        let mut by_sender = self.by_sender.write();

        // Collect entries to remove
        let to_remove: Vec<_> = insertion_order
            .range(..cutoff)
            .map(|(time, hash)| (*time, *hash))
            .collect();

        for (time, hash) in to_remove {
            if let Some(entry) = messages.remove(&hash) {
                if let Some(sender) = entry.sender {
                    if let Some(hashes) = by_sender.get_mut(&sender) {
                        hashes.retain(|h| h != &hash);
                    }
                }
            }
            insertion_order.remove(&time);
        }
    }

    /// Get statistics about the store
    #[must_use]
    pub fn stats(&self) -> StoreStats {
        let messages = self.messages.read();
        let processed_count = messages.values().filter(|e| e.processed).count();

        StoreStats {
            total_messages: messages.len(),
            processed_messages: processed_count,
            unprocessed_messages: messages.len() - processed_count,
            unique_senders: self.by_sender.read().len(),
        }
    }
}

/// Statistics about the message store
#[derive(Debug, Clone, Default)]
pub struct StoreStats {
    /// Total number of messages in the store
    pub total_messages: usize,
    /// Number of processed messages
    pub processed_messages: usize,
    /// Number of unprocessed messages
    pub unprocessed_messages: usize,
    /// Number of unique senders
    pub unique_senders: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_hash(id: u8) -> MessageHash {
        let mut hash = [0u8; 32];
        hash[0] = id;
        hash
    }

    fn make_peer(id: u8) -> PeerId {
        let mut bytes = [0u8; 32];
        bytes[0] = id;
        PeerId::from_bytes(bytes)
    }

    #[test]
    fn test_store_insert_and_get() {
        let store: MessageStore<String> = MessageStore::with_defaults();
        let hash = make_hash(1);
        let entry = MessageEntry::new("test message".to_string(), hash, None);

        assert!(store.insert(entry));
        assert!(store.contains(&hash));

        let retrieved = store.get(&hash).unwrap();
        assert_eq!(retrieved.message, "test message");
    }

    #[test]
    fn test_store_duplicate_rejection() {
        let store: MessageStore<String> = MessageStore::with_defaults();
        let hash = make_hash(1);

        let entry1 = MessageEntry::new("first".to_string(), hash, None);
        let entry2 = MessageEntry::new("second".to_string(), hash, None);

        assert!(store.insert(entry1));
        assert!(!store.insert(entry2)); // Duplicate

        // Should still have the first message
        let retrieved = store.get(&hash).unwrap();
        assert_eq!(retrieved.message, "first");
    }

    #[test]
    fn test_store_by_sender() {
        let store: MessageStore<String> = MessageStore::with_defaults();
        let peer = make_peer(1);

        for i in 0..3 {
            let hash = make_hash(i);
            let entry = MessageEntry::new(format!("msg-{i}"), hash, Some(peer));
            store.insert(entry);
        }

        let from_peer = store.get_by_sender(&peer);
        assert_eq!(from_peer.len(), 3);
    }

    #[test]
    fn test_store_mark_processed() {
        let store: MessageStore<String> = MessageStore::with_defaults();
        let hash = make_hash(1);
        let entry = MessageEntry::new("test".to_string(), hash, None);

        store.insert(entry);
        assert!(!store.get(&hash).unwrap().processed);

        store.mark_processed(&hash);
        assert!(store.get(&hash).unwrap().processed);
    }

    #[test]
    fn test_store_eviction() {
        let config = StoreConfig {
            max_messages: 10,
            index_by_sender: true,
        };
        let store: MessageStore<String> = MessageStore::new(config);

        // Insert 15 messages
        for i in 0..15 {
            let hash = make_hash(i);
            let entry = MessageEntry::new(format!("msg-{i}"), hash, None);
            store.insert(entry);
        }

        // Should have evicted some
        assert!(store.len() <= 10);
    }

    #[test]
    fn test_store_remove() {
        let store: MessageStore<String> = MessageStore::with_defaults();
        let hash = make_hash(1);
        let entry = MessageEntry::new("test".to_string(), hash, None);

        store.insert(entry);
        assert!(store.contains(&hash));

        store.remove(&hash);
        assert!(!store.contains(&hash));
    }
}
