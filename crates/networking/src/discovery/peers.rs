use std::{
    collections::HashSet,
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};

use crate::InstanceMsgPublicKey;
use dashmap::{DashMap, DashSet};
use libp2p::{core::Multiaddr, identify, PeerId};
use tokio::sync::broadcast;
use tracing::debug;

/// Information about a peer's connection and behavior
#[derive(Clone, Debug)]
pub struct PeerInfo {
    /// Known addresses for the peer
    pub addresses: HashSet<Multiaddr>,
    /// Information from the identify protocol
    pub identify_info: Option<identify::Info>,
    /// When the peer was last seen
    pub last_seen: SystemTime,
    /// Latest ping latency
    pub ping_latency: Option<Duration>,
    /// Number of successful protocol interactions
    pub successes: u32,
    /// Number of failed protocol interactions
    pub failures: u32,
    /// Average response time for protocol requests
    pub average_response_time: Option<Duration>,
}

impl Default for PeerInfo {
    fn default() -> Self {
        Self {
            addresses: HashSet::new(),
            identify_info: None,
            last_seen: SystemTime::now(),
            ping_latency: None,
            successes: 0,
            failures: 0,
            average_response_time: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum PeerEvent {
    /// A peer was added or updated
    PeerUpdated { peer_id: PeerId, info: Box<PeerInfo> },
    /// A peer was removed
    PeerRemoved { peer_id: PeerId, reason: String },
    /// A peer was banned
    PeerBanned {
        peer_id: PeerId,
        reason: String,
        expires_at: Option<Instant>,
    },
    /// A peer was unbanned
    PeerUnbanned { peer_id: PeerId },
}

pub struct PeerManager {
    /// Active peers and their information
    peers: DashMap<PeerId, PeerInfo>,
    /// Verified peers from completed handshakes
    verified_peers: DashSet<PeerId>,
    /// Handshake keys to peer ids
    public_keys_to_peer_ids: Arc<DashMap<InstanceMsgPublicKey, PeerId>>,
    /// Banned peers with optional expiration time
    banned_peers: DashMap<PeerId, Option<Instant>>,
    /// Allowed public keys
    whitelisted_keys: DashSet<InstanceMsgPublicKey>,
    /// Event sender for peer updates
    event_tx: broadcast::Sender<PeerEvent>,
}

impl Default for PeerManager {
    fn default() -> Self {
        Self::new(HashSet::default())
    }
}

impl PeerManager {
    #[must_use]
    pub fn new(whitelisted_keys: HashSet<InstanceMsgPublicKey>) -> Self {
        let (event_tx, _) = broadcast::channel(100);
        Self {
            peers: DashMap::default(),
            banned_peers: DashMap::default(),
            verified_peers: DashSet::default(),
            public_keys_to_peer_ids: Arc::new(DashMap::default()),
            whitelisted_keys: DashSet::from_iter(whitelisted_keys),
            event_tx,
        }
    }

    pub fn update_whitelisted_keys(&self, keys: HashSet<InstanceMsgPublicKey>) {
        self.whitelisted_keys.clear();
        for key in keys {
            self.whitelisted_keys.insert(key);
        }
    }

    #[must_use]
    pub fn is_key_whitelisted(&self, key: &InstanceMsgPublicKey) -> bool {
        self.whitelisted_keys.contains(key)
    }

    pub fn handle_nonwhitelisted_peer(&self, peer: &PeerId) {
        self.remove_peer(peer, "non-whitelisted");
        self.ban_peer(*peer, "non-whitelisted", None);
    }

    /// Get a subscription to peer events
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<PeerEvent> {
        self.event_tx.subscribe()
    }

    /// Update or add peer information
    pub fn update_peer(&self, peer_id: PeerId, mut info: PeerInfo) {
        // Update last seen time
        info.last_seen = SystemTime::now();

        // Insert or update peer info
        self.peers.insert(peer_id, info.clone());

        // Emit event
        let _ = self.event_tx.send(PeerEvent::PeerUpdated { peer_id, info: Box::new(info) });
    }

    /// Remove a peer
    pub fn remove_peer(&self, peer_id: &PeerId, reason: impl Into<String>) {
        if self.peers.remove(peer_id).is_some() {
            let reason = reason.into();
            debug!(%peer_id, %reason, "removed peer");
            let _ = self.event_tx.send(PeerEvent::PeerRemoved {
                peer_id: *peer_id,
                reason,
            });
        }
    }

    /// Verify a peer
    pub fn verify_peer(&self, peer_id: &PeerId) {
        self.verified_peers.insert(*peer_id);
    }

    /// Check if a peer is verified
    #[must_use]
    pub fn is_peer_verified(&self, peer_id: &PeerId) -> bool {
        self.verified_peers.contains(peer_id)
    }

    /// Ban a peer with optional expiration
    pub fn ban_peer(&self, peer_id: PeerId, reason: impl Into<String>, duration: Option<Duration>) {
        let expires_at = duration.map(|d| Instant::now() + d);

        // Remove from active peers
        self.remove_peer(&peer_id, "banned");

        // Add to banned peers
        self.banned_peers.insert(peer_id, expires_at);

        let reason = reason.into();
        debug!(%peer_id, %reason, "banned peer");
        let _ = self.event_tx.send(PeerEvent::PeerBanned {
            peer_id,
            reason,
            expires_at,
        });
    }

    /// Bans a peer with the default duration(`1h`)
    pub fn ban_peer_with_default_duration(&self, peer: PeerId, reason: impl Into<String>) {
        const BAN_PEER_DURATION: Duration = Duration::from_secs(60 * 60); //1h
        self.ban_peer(peer, reason, Some(BAN_PEER_DURATION));
    }

    /// Unban a peer
    pub fn unban_peer(&self, peer_id: &PeerId) {
        if self.banned_peers.remove(peer_id).is_some() {
            debug!(%peer_id, "unbanned peer");
            let _ = self
                .event_tx
                .send(PeerEvent::PeerUnbanned { peer_id: *peer_id });
        }
    }

    /// Check if a peer is banned
    #[must_use]
    pub fn is_banned(&self, peer_id: &PeerId) -> bool {
        self.banned_peers.contains_key(peer_id)
    }

    /// Log a successful interaction with a peer
    pub fn log_success(&self, peer_id: &PeerId, duration: Duration) {
        if let Some(mut info) = self.peers.get_mut(peer_id) {
            info.successes += 1;
            update_average_time(&mut info, duration);
            self.update_peer(*peer_id, info.clone());
        }
    }

    /// Log a failed interaction with a peer
    pub fn log_failure(&self, peer_id: &PeerId, duration: Duration) {
        if let Some(mut info) = self.peers.get_mut(peer_id) {
            info.failures += 1;
            update_average_time(&mut info, duration);
            self.update_peer(*peer_id, info.clone());
        }
    }

    /// Get peer information
    #[must_use]
    pub fn get_peer_info(&self, peer_id: &PeerId) -> Option<PeerInfo> {
        self.peers.get(peer_id).map(|info| info.value().clone())
    }

    /// Get all active peers
    #[must_use]
    pub fn get_peers(&self) -> DashMap<PeerId, PeerInfo> {
        self.peers.clone()
    }

    /// Get number of active peers
    #[must_use]
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// Start the background task to clean up expired bans
    pub async fn run_ban_cleanup(self: Arc<Self>) {
        loop {
            let now = Instant::now();
            let mut to_unban = Vec::new();

            // Find expired bans
            let banned_peers = self.banned_peers.clone().into_read_only();
            for (peer_id, expires_at) in banned_peers.iter() {
                if let Some(expiry) = expires_at {
                    if now >= *expiry {
                        to_unban.push(*peer_id);
                    }
                }
            }

            // Unban expired peers
            for peer_id in to_unban {
                self.unban_peer(&peer_id);
            }

            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }

    /// Add a peer id to the public key to peer id map after verifying handshake
    pub fn add_peer_id_to_public_key(&self, peer_id: &PeerId, public_key: &InstanceMsgPublicKey) {
        self.public_keys_to_peer_ids.insert(*public_key, *peer_id);
    }

    /// Remove a peer id from the public key to peer id map
    pub fn remove_peer_id_from_public_key(&self, peer_id: &PeerId) {
        self.public_keys_to_peer_ids.retain(|_, id| id != peer_id);
    }

    #[must_use]
    pub fn get_peer_id_from_public_key(&self, public_key: &InstanceMsgPublicKey) -> Option<PeerId> {
        self.public_keys_to_peer_ids.get(public_key).map(|id| *id)
    }
}

/// Update the average response time for a peer
fn update_average_time(info: &mut PeerInfo, duration: Duration) {
    const ALPHA: u32 = 5; // Smoothing factor for the moving average

    if info.average_response_time.is_none() {
        info.average_response_time = Some(duration);
    } else if duration < info.average_response_time.unwrap() {
        let delta = (info.average_response_time.unwrap() - duration) / ALPHA;
        info.average_response_time = Some(info.average_response_time.unwrap() - delta);
    } else {
        let delta = (duration - info.average_response_time.unwrap()) / ALPHA;
        info.average_response_time = Some(info.average_response_time.unwrap() + delta);
    }
}
