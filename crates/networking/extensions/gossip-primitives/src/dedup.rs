//! Message deduplication for gossip protocols
//!
//! This module provides [`GossipManager`] which handles:
//! - Message deduplication with bounded LRU cache
//! - Time-based expiration of seen messages
//! - Statistics tracking for monitoring

use alloc::vec::Vec;
use core::num::NonZeroUsize;
use core::time::Duration;
use lru::LruCache;
use parking_lot::Mutex;

use crate::clock::Instant;

/// Hash type for message deduplication (32 bytes, typically blake3)
pub type MessageHash = [u8; 32];

/// Configuration for the gossip manager
#[derive(Debug, Clone)]
pub struct GossipConfig {
    /// Maximum number of message hashes to cache for deduplication
    pub max_seen_messages: usize,
    /// How long to remember a message hash before allowing re-processing
    pub message_ttl: Duration,
    /// Whether to track statistics
    pub enable_stats: bool,
}

impl Default for GossipConfig {
    fn default() -> Self {
        Self {
            max_seen_messages: 10_000,
            message_ttl: Duration::from_secs(300), // 5 minutes
            enable_stats: true,
        }
    }
}

impl GossipConfig {
    /// Create a config suitable for testing (smaller cache, shorter TTL)
    #[must_use]
    pub fn for_testing() -> Self {
        Self {
            max_seen_messages: 100,
            message_ttl: Duration::from_secs(30),
            enable_stats: true,
        }
    }
}

/// Entry in the deduplication cache
#[derive(Debug, Clone)]
struct CacheEntry {
    /// When the message was first seen
    first_seen: Instant,
    /// How many times we've seen this message
    seen_count: u32,
}

/// LRU-based deduplication cache with time expiration
pub struct DeduplicationCache {
    cache: Mutex<LruCache<MessageHash, CacheEntry>>,
    ttl: Duration,
}

impl DeduplicationCache {
    /// Create a new deduplication cache
    #[must_use]
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        let capacity = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(1).unwrap());
        Self {
            cache: Mutex::new(LruCache::new(capacity)),
            ttl,
        }
    }

    /// Check if a message should be processed (not a duplicate)
    ///
    /// Returns `true` if the message is new or expired and should be processed.
    /// Returns `false` if the message is a recent duplicate.
    pub fn should_process(&self, hash: &MessageHash) -> bool {
        let mut cache = self.cache.lock();

        if let Some(entry) = cache.get(hash) {
            // Check if entry has expired
            if entry.first_seen.elapsed() > self.ttl {
                // Expired, treat as new
                cache.pop(hash);
                true
            } else {
                // Still valid, this is a duplicate
                false
            }
        } else {
            // Never seen before
            true
        }
    }

    /// Mark a message as seen
    ///
    /// Should be called after successfully processing a message.
    pub fn mark_seen(&self, hash: MessageHash) {
        let mut cache = self.cache.lock();

        if let Some(entry) = cache.get_mut(&hash) {
            entry.seen_count = entry.seen_count.saturating_add(1);
        } else {
            cache.put(
                hash,
                CacheEntry {
                    first_seen: Instant::now(),
                    seen_count: 1,
                },
            );
        }
    }

    /// Check and mark in one operation (atomic)
    ///
    /// Returns `true` if the message was new and has been marked.
    /// Returns `false` if the message was a duplicate.
    pub fn check_and_mark(&self, hash: MessageHash) -> bool {
        let mut cache = self.cache.lock();

        if let Some(entry) = cache.get_mut(&hash) {
            if entry.first_seen.elapsed() > self.ttl {
                // Expired, refresh
                *entry = CacheEntry {
                    first_seen: Instant::now(),
                    seen_count: 1,
                };
                true
            } else {
                // Duplicate
                entry.seen_count = entry.seen_count.saturating_add(1);
                false
            }
        } else {
            // New message
            cache.put(
                hash,
                CacheEntry {
                    first_seen: Instant::now(),
                    seen_count: 1,
                },
            );
            true
        }
    }

    /// Get the number of cached entries
    #[must_use]
    pub fn len(&self) -> usize {
        self.cache.lock().len()
    }

    /// Check if the cache is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cache.lock().is_empty()
    }

    /// Clear the cache
    pub fn clear(&self) {
        self.cache.lock().clear();
    }

    /// Remove expired entries (garbage collection)
    pub fn gc(&self) {
        let mut cache = self.cache.lock();
        let expired: Vec<_> = cache
            .iter()
            .filter(|(_, entry)| entry.first_seen.elapsed() > self.ttl)
            .map(|(hash, _)| *hash)
            .collect();

        for hash in expired {
            cache.pop(&hash);
        }
    }
}

/// Statistics for gossip operations
#[derive(Debug, Clone, Default)]
pub struct GossipStats {
    /// Total messages processed
    pub messages_processed: u64,
    /// Messages rejected as duplicates
    pub duplicates_rejected: u64,
    /// Messages re-gossiped
    pub messages_regossiped: u64,
    /// Send failures
    pub send_failures: u64,
}

/// Manager for gossip message handling with deduplication
pub struct GossipManager {
    /// Deduplication cache
    cache: DeduplicationCache,
    /// Configuration
    config: GossipConfig,
    /// Statistics (if enabled)
    stats: Option<Mutex<GossipStats>>,
}

impl GossipManager {
    /// Create a new gossip manager with the given configuration
    #[must_use]
    pub fn new(config: GossipConfig) -> Self {
        let cache = DeduplicationCache::new(config.max_seen_messages, config.message_ttl);
        let stats = if config.enable_stats {
            Some(Mutex::new(GossipStats::default()))
        } else {
            None
        };

        Self {
            cache,
            config,
            stats,
        }
    }

    /// Check if a message should be processed (not a duplicate)
    #[must_use]
    pub fn should_process(&self, hash: &MessageHash) -> bool {
        self.cache.should_process(hash)
    }

    /// Mark a message as seen and processed
    pub fn mark_processed(&self, hash: MessageHash) {
        self.cache.mark_seen(hash);
        if let Some(ref stats) = self.stats {
            stats.lock().messages_processed += 1;
        }
    }

    /// Record that a message was rejected as a duplicate
    pub fn record_duplicate(&self) {
        if let Some(ref stats) = self.stats {
            stats.lock().duplicates_rejected += 1;
        }
    }

    /// Record that a message was re-gossiped
    pub fn record_regossip(&self) {
        if let Some(ref stats) = self.stats {
            stats.lock().messages_regossiped += 1;
        }
    }

    /// Record a send failure
    pub fn record_send_failure(&self) {
        if let Some(ref stats) = self.stats {
            stats.lock().send_failures += 1;
        }
    }

    /// Get current statistics
    #[must_use]
    pub fn stats(&self) -> Option<GossipStats> {
        self.stats.as_ref().map(|s| s.lock().clone())
    }

    /// Get the deduplication cache
    #[must_use]
    pub fn cache(&self) -> &DeduplicationCache {
        &self.cache
    }

    /// Get the configuration
    #[must_use]
    pub fn config(&self) -> &GossipConfig {
        &self.config
    }

    /// Run garbage collection on the cache
    pub fn gc(&self) {
        self.cache.gc();
    }

    /// Compute a message hash using blake3
    #[must_use]
    pub fn hash_message(data: &[u8]) -> MessageHash {
        *blake3::hash(data).as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dedup_cache_basic() {
        let cache = DeduplicationCache::new(100, Duration::from_secs(60));
        let hash = [1u8; 32];

        // First time should be processable
        assert!(cache.should_process(&hash));

        // Mark as seen
        cache.mark_seen(hash);

        // Now should be duplicate
        assert!(!cache.should_process(&hash));
    }

    #[test]
    fn test_dedup_cache_check_and_mark() {
        let cache = DeduplicationCache::new(100, Duration::from_secs(60));
        let hash = [2u8; 32];

        // First time should succeed
        assert!(cache.check_and_mark(hash));

        // Second time should fail (duplicate)
        assert!(!cache.check_and_mark(hash));
    }

    #[test]
    fn test_gossip_manager_stats() {
        let config = GossipConfig::for_testing();
        let manager = GossipManager::new(config);

        let hash = GossipManager::hash_message(b"test message");

        // Process message
        assert!(manager.should_process(&hash));
        manager.mark_processed(hash);

        // Try duplicate
        assert!(!manager.should_process(&hash));
        manager.record_duplicate();

        let stats = manager.stats().unwrap();
        assert_eq!(stats.messages_processed, 1);
        assert_eq!(stats.duplicates_rejected, 1);
    }

    #[test]
    fn test_lru_eviction() {
        let cache = DeduplicationCache::new(3, Duration::from_secs(60));

        // Add 3 entries
        for i in 0..3 {
            let mut hash = [0u8; 32];
            hash[0] = i;
            cache.mark_seen(hash);
        }

        assert_eq!(cache.len(), 3);

        // Add 4th entry, should evict oldest
        let mut new_hash = [0u8; 32];
        new_hash[0] = 10;
        cache.mark_seen(new_hash);

        assert_eq!(cache.len(), 3);

        // First entry should have been evicted
        let mut first_hash = [0u8; 32];
        first_hash[0] = 0;
        assert!(cache.should_process(&first_hash)); // Can process again because evicted
    }
}
