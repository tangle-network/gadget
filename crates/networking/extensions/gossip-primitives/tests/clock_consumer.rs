//! Real public-crate consumer: no Tokio runtime or clock replacement is installed.
use blueprint_gossip_primitives::store::StoreConfig;
use blueprint_gossip_primitives::{DeduplicationCache, MessageEntry, MessageStore};
use std::{thread, time::Duration};

#[test]
fn public_clock_preserves_age_ttl_and_oldest_entry_eviction() {
    let store = MessageStore::new(StoreConfig {
        max_messages: 2,
        index_by_sender: false,
    });
    let first = MessageEntry::new(1_u8, [1; 32], None);
    store.insert(first.clone());
    thread::sleep(Duration::from_millis(2));
    store.insert(MessageEntry::new(2, [2; 32], None));
    thread::sleep(Duration::from_millis(2));
    store.insert(MessageEntry::new(3, [3; 32], None));

    assert!(first.age() > Duration::ZERO);
    assert_eq!(store.len(), 2);
    assert!(
        !store.contains(&[1; 32]),
        "capacity eviction removes the oldest entry"
    );
    assert!(store.contains(&[2; 32]));
    assert!(store.contains(&[3; 32]));

    let cache = DeduplicationCache::new(2, Duration::ZERO);
    assert!(cache.check_and_mark([7; 32]));
    assert_eq!(cache.len(), 1);
    thread::sleep(Duration::from_millis(2));
    assert!(
        cache.should_process(&[7; 32]),
        "elapsed time expires a real cache entry"
    );
}
