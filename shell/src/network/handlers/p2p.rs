use crate::network::gossip::MyBehaviourRequest::Handshake;
use crate::network::gossip::MyBehaviourResponse::{Handshaked, MessageHandled};
use crate::network::gossip::{MyBehaviourRequest, MyBehaviourResponse, NetworkService};
use libp2p::gossipsub::IdentTopic;
use libp2p::request_response::Event::{InboundFailure, Message, OutboundFailure, ResponseSent};
use libp2p::request_response::Message::{Request, Response};
use libp2p::{request_response, PeerId};

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
            Handshaked { ecdsa_public_key } => {
                // Their response to our handshake request.
                // we should add them to our mapping.
                // TODO: Add signature verification here.
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
            Handshake { ecdsa_public_key } => {
                self.logger
                    .debug(format!("Received handshake from peer: {peer}"));
                self.ecdsa_peer_id_to_libp2p_id
                    .write()
                    .await
                    .insert(ecdsa_public_key, peer);
                self.swarm.behaviour_mut().p2p.send_response(
                    channel,
                    MyBehaviourResponse::Handshaked {
                        ecdsa_public_key: *self.role_key,
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
