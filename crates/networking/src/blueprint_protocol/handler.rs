use std::time::{Duration, Instant};

use libp2p::{request_response, PeerId};
use tracing::{debug, warn};

use crate::blueprint_protocol::HandshakeMessage;
use crate::{key_types::InstanceMsgPublicKey, types::ProtocolMessage};

use super::{BlueprintProtocolBehaviour, InstanceMessageRequest, InstanceMessageResponse};

const INBOUND_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);
const OUTBOUND_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);

impl BlueprintProtocolBehaviour {
    #[allow(clippy::too_many_lines)]
    pub fn handle_request_response_event(
        &mut self,
        event: request_response::Event<InstanceMessageRequest, InstanceMessageResponse>,
    ) {
        match event {
            request_response::Event::Message {
                peer,
                message:
                    request_response::Message::Request {
                        request:
                            InstanceMessageRequest::Handshake {
                                public_key,
                                signature,
                                msg,
                            },
                        channel,
                        ..
                    },
                ..
            } => {
                debug!(%peer, "Received handshake request");

                // Check if we already sent a handshake request to this peer
                if self.outbound_handshakes.contains_key(&peer) {
                    // If we have an outbound handshake pending, we should still respond to their request
                    // This ensures both sides complete their handshakes even if messages cross on the wire
                    debug!(%peer, "Responding to inbound handshake request while outbound is pending");
                }

                if !self.peer_manager.is_key_whitelisted(&public_key) {
                    warn!(%peer, %public_key, "Received handshake response from unwhitelisted peer");
                    self.peer_manager.handle_nonwhitelisted_peer(&peer);
                    return;
                }

                // Verify the handshake
                match self.verify_handshake(&msg, &public_key, &signature) {
                    Ok(()) => {
                        // Store the handshake request
                        self.inbound_handshakes.insert(peer, Instant::now());
                        self.peer_manager
                            .add_peer_id_to_public_key(&peer, &public_key);

                        // Send handshake response
                        let mut key_pair = self.instance_key_pair.clone();

                        let handshake_msg = HandshakeMessage::new(self.local_peer_id);
                        let Some(signature) =
                            self.sign_handshake(&mut key_pair, &peer, &handshake_msg)
                        else {
                            return;
                        };

                        let response = InstanceMessageResponse::Handshake {
                            public_key: key_pair.public(),
                            signature,
                            msg: handshake_msg,
                        };

                        if let Err(e) = self.send_response(channel, response) {
                            warn!(%peer, "Failed to send handshake response: {:?}", e);
                            return;
                        }
                    }
                    Err(e) => {
                        warn!(%peer, "Invalid handshake request: {:?}", e);
                        let response = InstanceMessageResponse::Error {
                            code: 400,
                            message: format!("Invalid handshake: {:?}", e),
                        };
                        if let Err(e) = self.send_response(channel, response) {
                            warn!(%peer, "Failed to send error response: {:?}", e);
                        }
                    }
                }
            }
            request_response::Event::Message {
                peer,
                message:
                    request_response::Message::Response {
                        response:
                            InstanceMessageResponse::Handshake {
                                public_key,
                                signature,
                                msg,
                            },
                        ..
                    },
                ..
            } => {
                debug!(%peer, "Received handshake response");

                // Verify we have a pending outbound handshake
                if !self.outbound_handshakes.contains_key(&peer) {
                    warn!(%peer, "Received unexpected handshake response");
                    return;
                }

                if !self.peer_manager.is_key_whitelisted(&public_key) {
                    warn!(%peer, "Received handshake response from unwhitelisted peer");
                    self.peer_manager.handle_nonwhitelisted_peer(&peer);
                    return;
                }

                // Verify the handshake
                match self.verify_handshake(&msg, &public_key, &signature) {
                    Ok(()) => {
                        // Mark handshake as completed
                        self.complete_handshake(&peer, &public_key);
                    }
                    Err(e) => {
                        warn!(%peer, "Invalid handshake verification: {:?}", e);
                        self.outbound_handshakes.remove(&peer);
                        self.handle_handshake_failure(&peer, "Invalid handshake verification");
                    }
                }
            }
            request_response::Event::Message {
                peer,
                message:
                    request_response::Message::Request {
                        request:
                            InstanceMessageRequest::Protocol {
                                protocol,
                                payload,
                                metadata: _,
                            },
                        channel,
                        ..
                    },
                ..
            } => {
                // Reject messages from self
                if peer == self.local_peer_id {
                    return;
                }

                // Only accept protocol messages from peers we've completed handshakes with
                if !self.peer_manager.is_peer_verified(&peer) {
                    warn!(%peer, "Received protocol message from unverified peer");
                    let response = InstanceMessageResponse::Error {
                        code: 403,
                        message: "Handshake required".to_string(),
                    };
                    if let Err(e) = self.send_response(channel, response) {
                        warn!(%peer, "Failed to send error response: {:?}", e);
                    }
                    return;
                }

                let protocol_message: ProtocolMessage = match bincode::deserialize(&payload) {
                    Ok(message) => message,
                    Err(e) => {
                        warn!(%peer, "Failed to deserialize protocol message: {:?}", e);
                        let response = InstanceMessageResponse::Error {
                            code: 400,
                            message: format!("Invalid protocol message: {:?}", e),
                        };
                        if let Err(e) = self.send_response(channel, response) {
                            warn!(%peer, "Failed to send error response: {:?}", e);
                        }
                        return;
                    }
                };

                debug!(%peer, %protocol, %protocol_message, "Received protocol request");
                if let Err(e) = self.protocol_message_sender.send(protocol_message) {
                    warn!(%peer, "Failed to send protocol message: {:?}", e);
                }
            }
            request_response::Event::Message {
                peer,
                message:
                    request_response::Message::Response {
                        response: InstanceMessageResponse::Error { code, message },
                        ..
                    },
                ..
            } => {
                if !self.peer_manager.is_peer_verified(&peer) {
                    warn!(%peer, code, %message, "Received error response from unverified peer");
                    return;
                }
            }
            request_response::Event::Message {
                peer,
                message:
                    request_response::Message::Response {
                        response: InstanceMessageResponse::Success { protocol, data: _ },
                        ..
                    },
                ..
            } => {
                debug!(%peer, %protocol, "Received successful protocol response");
            }
            _ => {}
        }

        // Check for expired handshakes
        self.check_expired_handshakes();
    }

    /// Check for and remove expired handshakes
    fn check_expired_handshakes(&mut self) {
        let now = Instant::now();

        // Check inbound handshakes
        let expired_inbound: Vec<_> = self
            .inbound_handshakes
            .clone()
            .into_read_only()
            .iter()
            .filter(|(_, &time)| now.duration_since(time) > INBOUND_HANDSHAKE_TIMEOUT)
            .map(|(peer_id, _)| *peer_id)
            .collect();

        for peer_id in expired_inbound {
            self.inbound_handshakes.remove(&peer_id);
            self.handle_handshake_failure(&peer_id, "Inbound handshake timeout");
        }

        // Check outbound handshakes
        let expired_outbound: Vec<_> = self
            .outbound_handshakes
            .clone()
            .into_read_only()
            .iter()
            .filter(|(_, &time)| now.duration_since(time) > OUTBOUND_HANDSHAKE_TIMEOUT)
            .map(|(peer_id, _)| *peer_id)
            .collect();

        for peer_id in expired_outbound {
            self.outbound_handshakes.remove(&peer_id);
            self.handle_handshake_failure(&peer_id, "Outbound handshake timeout");
        }
    }

    /// Complete a successful handshake
    fn complete_handshake(&mut self, peer: &PeerId, public_key: &InstanceMsgPublicKey) {
        debug!(%peer, "Completed handshake");

        // Remove from pending handshakes
        self.inbound_handshakes.remove(peer);
        self.outbound_handshakes.remove(peer);

        // Update peer manager
        self.peer_manager
            .add_peer_id_to_public_key(peer, public_key);

        // Add to verified peers
        self.peer_manager.verify_peer(peer);
    }
}
