use std::{num::NonZero, time::Duration};

use libp2p::{
    kad::{self, store::MemoryStore},
    PeerId, StreamProtocol,
};

pub mod behaviour;
pub mod config;
pub mod peers;

pub use peers::{PeerEvent, PeerInfo, PeerManager};

const MAX_ESTABLISHED_PER_PEER: u32 = 4;

pub fn new_kademlia(peer_id: PeerId, protocol: StreamProtocol) -> kad::Behaviour<MemoryStore> {
    let store = kad::store::MemoryStore::new(peer_id);
    let mut config = kad::Config::new(protocol);

    // Optimize Kademlia configuration
    config
        .set_query_timeout(Duration::from_secs(60))
        .set_replication_factor(NonZero::new(3).unwrap())
        .set_publication_interval(Some(Duration::from_secs(120)))
        .set_provider_record_ttl(Some(Duration::from_secs(24 * 60 * 60)))
        .set_record_ttl(Some(Duration::from_secs(24 * 60 * 60)))
        .set_parallelism(NonZero::new(5).unwrap());

    let mut kademlia = kad::Behaviour::with_config(peer_id, store, config);
    kademlia.set_mode(Some(kad::Mode::Server));
    kademlia
}
