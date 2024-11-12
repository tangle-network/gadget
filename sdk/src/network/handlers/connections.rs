#![allow(unused_results, clippy::used_underscore_binding)]

use crate::network::gossip::{MyBehaviourRequest, NetworkService};
use crate::{error, trace, warn};
use itertools::Itertools;
use libp2p::PeerId;
use sp_core::{keccak_256, Pair};

impl NetworkService<'_> {
    #[tracing::instrument(skip(self))]
    pub(crate) async fn handle_connection_established(
        &mut self,
        peer_id: PeerId,
        num_established: u32,
    ) {
        crate::debug!("Connection established");
        if num_established == 1 {
            let my_peer_id = self.swarm.local_peer_id();
            let msg = my_peer_id.to_bytes();
            let hash = keccak_256(&msg);
            let signature = self.ecdsa_key.sign_prehashed(&hash);
            let handshake = MyBehaviourRequest::Handshake {
                ecdsa_public_key: self.ecdsa_key.public(),
                signature,
            };
            self.swarm
                .behaviour_mut()
                .p2p
                .send_request(&peer_id, handshake);
            self.swarm
                .behaviour_mut()
                .gossipsub
                .add_explicit_peer(&peer_id);
        }
    }

    #[tracing::instrument(skip(self))]
    pub(crate) async fn handle_connection_closed(
        &mut self,
        peer_id: PeerId,
        num_established: u32,
        _cause: Option<libp2p::swarm::ConnectionError>,
    ) {
        trace!("Connection closed");
        if num_established == 0 {
            self.swarm
                .behaviour_mut()
                .gossipsub
                .remove_explicit_peer(&peer_id);
        }
    }

    #[tracing::instrument(skip(self))]
    pub(crate) async fn handle_incoming_connection(
        &mut self,
        _connection_id: libp2p::swarm::ConnectionId,
        _local_addr: libp2p::Multiaddr,
        _send_back_addr: libp2p::Multiaddr,
    ) {
        trace!("Incoming connection");
    }

    #[tracing::instrument(skip(self))]
    async fn handle_outgoing_connection(
        &mut self,
        peer_id: PeerId,
        _connection_id: libp2p::swarm::ConnectionId,
    ) {
        trace!("Outgoing connection to peer: {peer_id}");
    }

    #[tracing::instrument(skip(self, error))]
    pub(crate) async fn handle_incoming_connection_error(
        &mut self,
        _connection_id: libp2p::swarm::ConnectionId,
        _local_addr: libp2p::Multiaddr,
        _send_back_addr: libp2p::Multiaddr,
        error: libp2p::swarm::ListenError,
    ) {
        error!("Incoming connection error: {error}");
    }

    #[tracing::instrument(skip(self, error))]
    pub(crate) async fn handle_outgoing_connection_error(
        &mut self,
        _connection_id: libp2p::swarm::ConnectionId,
        _peer_id: Option<PeerId>,
        error: libp2p::swarm::DialError,
    ) {
        if let libp2p::swarm::DialError::Transport(addrs) = error {
            let read = self.ecdsa_peer_id_to_libp2p_id.read().await;
            for (addr, err) in addrs {
                if let Some(peer_id) = get_peer_id_from_multiaddr(&addr) {
                    if !read.values().contains(&peer_id) {
                        warn!(
                            "Outgoing connection error to peer: {peer_id} at {addr}: {err}",
                            peer_id = peer_id,
                            addr = addr,
                            err = err
                        );
                    }
                }
            }
        } else {
            error!("Outgoing connection error to peer: {error}");
        }
    }
}

fn get_peer_id_from_multiaddr(addr: &libp2p::Multiaddr) -> Option<PeerId> {
    addr.iter()
        .find_map(|proto| {
            if let libp2p::multiaddr::Protocol::P2p(peer_id) = proto {
                Some(Some(peer_id))
            } else {
                None
            }
        })
        .flatten()
}
