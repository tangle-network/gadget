#![allow(unused_results, clippy::used_underscore_binding)]

use crate::gossip::{MyBehaviourRequest, NetworkService};
use gadget_crypto::{hashing::keccak_256, k256_crypto::K256Ecdsa, KeyType};
use itertools::Itertools;
use libp2p::PeerId;

impl NetworkService<'_> {
    #[tracing::instrument(skip(self))]
    pub(crate) async fn handle_connection_established(
        &mut self,
        peer_id: PeerId,
        num_established: u32,
    ) {
        gadget_logging::debug!("Connection established");
        if num_established == 1 {
            let my_peer_id = self.swarm.local_peer_id();
            let msg = my_peer_id.to_bytes();
            let hash = keccak_256(&msg);
            match <K256Ecdsa as KeyType>::sign_with_secret_pre_hashed(
                &mut self.ecdsa_secret_key.clone(),
                &hash,
            ) {
                Ok(signature) => {
                    let handshake = MyBehaviourRequest::Handshake {
                        ecdsa_public_key: self.ecdsa_secret_key.verifying_key(),
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
                Err(e) => {
                    gadget_logging::error!("Failed to sign handshake: {e}");
                }
            }
        }
    }

    #[tracing::instrument(skip(self))]
    pub(crate) async fn handle_connection_closed(
        &mut self,
        peer_id: PeerId,
        num_established: u32,
        _cause: Option<libp2p::swarm::ConnectionError>,
    ) {
        gadget_logging::trace!("Connection closed");
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
        gadget_logging::trace!("Incoming connection");
    }

    #[tracing::instrument(skip(self))]
    async fn handle_outgoing_connection(
        &mut self,
        peer_id: PeerId,
        _connection_id: libp2p::swarm::ConnectionId,
    ) {
        gadget_logging::trace!("Outgoing connection to peer: {peer_id}");
    }

    #[tracing::instrument(skip(self, error))]
    pub(crate) async fn handle_incoming_connection_error(
        &mut self,
        _connection_id: libp2p::swarm::ConnectionId,
        _local_addr: libp2p::Multiaddr,
        _send_back_addr: libp2p::Multiaddr,
        error: libp2p::swarm::ListenError,
    ) {
        gadget_logging::error!("Incoming connection error: {error}");
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
                        gadget_logging::warn!(
                            "Outgoing connection error to peer: {peer_id} at {addr}: {err}",
                            peer_id = peer_id,
                            addr = addr,
                            err = err
                        );
                    }
                }
            }
        } else {
            gadget_logging::error!("Outgoing connection error to peer: {error}");
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
