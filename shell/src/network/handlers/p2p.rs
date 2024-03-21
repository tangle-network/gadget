use crate::network::gossip::{MyBehaviourRequest, MyBehaviourResponse, NetworkService};
use libp2p::gossipsub::IdentTopic;

use libp2p::{request_response, PeerId};
use sp_core::{keccak_256, Pair};
use sp_io::crypto::ecdsa_verify_prehashed;

impl NetworkService<'_> {
    #[tracing::instrument(skip(self, event))]
    pub(crate) async fn handle_p2p(
        &mut self,
        event: request_response::Event<MyBehaviourRequest, MyBehaviourResponse>,
    ) {
        use request_response::Event::*;
        match event {
            Message { peer, message } => {
                self.logger
                    .debug(format!("Received P2P message from: {peer}"));
                self.handle_p2p_message(peer, message).await;
            }
            OutboundFailure {
                peer,
                request_id,
                error,
            } => {
                self.logger.error(format!("Failed to send message to peer: {peer} with request_id: {request_id} and error: {error}"));
            }
            InboundFailure {
                peer,
                request_id,
                error,
            } => {
                self.logger.error(format!("Failed to receive message from peer: {peer} with request_id: {request_id} and error: {error}"));
            }
            ResponseSent { peer, request_id } => {
                self.logger.debug(format!(
                    "Sent response to peer: {peer} with request_id: {request_id}"
                ));
            }
        }
    }

    #[tracing::instrument(skip(self, message))]
    async fn handle_p2p_message(
        &mut self,
        peer: PeerId,
        message: request_response::Message<MyBehaviourRequest, MyBehaviourResponse>,
    ) {
        use request_response::Message::*;
        match message {
            Request {
                request,
                channel,
                request_id,
            } => {
                self.logger.debug(format!(
                    "Received request with request_id: {request_id} from peer: {peer}"
                ));
                self.handle_p2p_request(peer, request_id, request, channel)
                    .await;
            }
            Response {
                response,
                request_id,
            } => {
                self.logger.debug(format!(
                    "Received response from peer: {peer} with request_id: {request_id}"
                ));
                self.handle_p2p_response(peer, request_id, response).await;
            }
        }
    }

    #[tracing::instrument(skip(self, message))]
    async fn handle_p2p_response(
        &mut self,
        peer: PeerId,
        _request_id: request_response::OutboundRequestId,
        message: MyBehaviourResponse,
    ) {
        use crate::network::gossip::MyBehaviourResponse::*;
        match message {
            Handshaked {
                ecdsa_public_key,
                signature,
            } => {
                let msg = peer.to_bytes();
                let hash = keccak_256(&msg);
                let valid = ecdsa_verify_prehashed(&signature, &hash, &ecdsa_public_key);
                if !valid {
                    self.logger
                        .warn(format!("Invalid signature from peer: {peer}"));
                    // TODO: report this peer.
                    self.ecdsa_peer_id_to_libp2p_id
                        .write()
                        .await
                        .remove(&ecdsa_public_key);
                    let _ = self.swarm.disconnect_peer_id(peer);
                    return;
                }
                self.ecdsa_peer_id_to_libp2p_id
                    .write()
                    .await
                    .insert(ecdsa_public_key, peer);
            }
            MessageHandled => {}
        }
    }

    #[tracing::instrument(skip(self, req, channel))]
    async fn handle_p2p_request(
        &mut self,
        peer: PeerId,
        request_id: request_response::InboundRequestId,
        req: MyBehaviourRequest,
        channel: request_response::ResponseChannel<MyBehaviourResponse>,
    ) {
        use crate::network::gossip::MyBehaviourRequest::*;
        let result = match req {
            Handshake {
                ecdsa_public_key,
                signature,
            } => {
                self.logger
                    .debug(format!("Received handshake from peer: {peer}"));
                // Verify the signature
                let msg = peer.to_bytes();
                let hash = keccak_256(&msg);
                let valid = ecdsa_verify_prehashed(&signature, &hash, &ecdsa_public_key);
                if !valid {
                    self.logger
                        .warn(format!("Invalid signature from peer: {peer}"));
                    let _ = self.swarm.disconnect_peer_id(peer);
                    return;
                }
                self.ecdsa_peer_id_to_libp2p_id
                    .write()
                    .await
                    .insert(ecdsa_public_key, peer);
                // Send response with our public key
                let my_peer_id = self.swarm.local_peer_id();
                let msg = my_peer_id.to_bytes();
                let hash = keccak_256(&msg);
                let signature = self.role_key.sign_prehashed(&hash);
                self.swarm.behaviour_mut().p2p.send_response(
                    channel,
                    MyBehaviourResponse::Handshaked {
                        ecdsa_public_key: self.role_key.public(),
                        signature,
                    },
                )
            }
            Message { topic, raw_payload } => {
                let topic = IdentTopic::new(topic);
                if let Some((_, tx, _)) = self
                    .inbound_mapping
                    .iter()
                    .find(|r| r.0.to_string() == topic.to_string())
                {
                    if let Err(e) = tx.send(raw_payload) {
                        self.logger
                            .error(format!("Failed to send message to worker: {e}"));
                    }
                } else {
                    self.logger
                        .error(format!("No registered worker for topic: {topic}!"));
                }
                self.swarm
                    .behaviour_mut()
                    .p2p
                    .send_response(channel, MyBehaviourResponse::MessageHandled)
            }
        };
        if result.is_err() {
            self.logger
                .error(format!("Failed to send response for {request_id}"));
        }
    }
}
