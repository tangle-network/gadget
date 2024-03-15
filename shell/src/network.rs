use crate::config::ShellConfig;
use crate::shell::{AGENT_VERSION, CLIENT_VERSION};
use async_trait::async_trait;
use futures::stream::StreamExt;
use gadget_common::prelude::{DebugLogger, Network, WorkManager};
use gadget_core::job_manager::WorkManagerInterface;
use libp2p::gossipsub::{IdentTopic, TopicHash};
use libp2p::kad::store::MemoryStore;
use libp2p::swarm::dial_opts::DialOpts;
use libp2p::{
    gossipsub, mdns, request_response, swarm::NetworkBehaviour, swarm::SwarmEvent, PeerId,
    StreamProtocol,
};
use serde::{Deserialize, Serialize};
use sp_core::ecdsa;
use std::collections::HashMap;
use std::error::Error;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use tokio::{io, select};

/// Maximum allowed size for a Signed Message.
pub const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;

// We create a custom network behaviour that combines Gossipsub and Mdns.
#[derive(NetworkBehaviour)]
struct MyBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
    p2p: request_response::cbor::Behaviour<MyBehaviourRequest, MyBehaviourResponse>,
    identify: libp2p::identify::Behaviour,
    kadmelia: libp2p::kad::Behaviour<MemoryStore>,
}

pub async fn setup_network(
    identity: libp2p::identity::Keypair,
    config: &ShellConfig,
    logger: DebugLogger,
    networks: Vec<&'static str>,
    role_key: ecdsa::Public,
) -> Result<(Vec<GossipHandle>, JoinHandle<()>), Box<dyn Error>> {
    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(identity)
        .with_tokio()
        /*.with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )?*/
        .with_quic_config(|mut config| {
            config.handshake_timeout = Duration::from_secs(30);
            config
        })
        .with_behaviour(|key| {
            // Set a custom gossipsub configuration
            let gossipsub_config = gossipsub::ConfigBuilder::default()
                .protocol_id_prefix("/tangle/gadget-shell/meshsub")
                .max_transmit_size(MAX_MESSAGE_SIZE)
                .validate_messages()
                .validation_mode(gossipsub::ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message signing)
                .build()
                .map_err(|msg| io::Error::new(io::ErrorKind::Other, msg))?; // Temporary hack because `build` does not return a proper `std::error::Error`.

            // Setup gossipsub network behaviour for broadcasting
            let gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                gossipsub_config,
            )?;

            // Setup mDNS for peer discovery
            let mdns =
                mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())?;

            // Setup request-response for direct messaging
            let p2p_config = request_response::Config::default();
            // StreamProtocols MUST begin with a forward slash
            let protocols = networks
                .iter()
                .map(|n| {
                    (
                        StreamProtocol::new(n),
                        request_response::ProtocolSupport::Full,
                    )
                })
                .collect::<Vec<_>>();

            let p2p = request_response::Behaviour::new(protocols, p2p_config);

            // Setup the identify protocol for peers to exchange information about each other, a requirement for kadmelia DHT
            let identify = libp2p::identify::Behaviour::new(
                libp2p::identify::Config::new(CLIENT_VERSION.into(), key.public())
                    .with_agent_version(AGENT_VERSION.into()),
            );

            // Setup kadmelia for DHT for peer discovery over a larger network
            let memory_db = MemoryStore::new(key.public().to_peer_id());
            let kadmelia = libp2p::kad::Behaviour::new(key.public().to_peer_id(), memory_db);

            Ok(MyBehaviour {
                gossipsub,
                mdns,
                p2p,
                identify,
                kadmelia,
            })
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    // Subscribe to all networks
    let mut inbound_mapping = Vec::new();
    let (tx_to_outbound, mut rx_to_outbound) =
        tokio::sync::mpsc::unbounded_channel::<IntraNodePayload>();
    let ecdsa_peer_id_to_libp2p_id = Arc::new(RwLock::new(HashMap::new()));
    let mut handles_ret = vec![];

    for network in networks {
        let topic = IdentTopic::new(network);
        swarm.behaviour_mut().gossipsub.subscribe(&topic)?;
        let (inbound_tx, inbound_rx) = tokio::sync::mpsc::unbounded_channel();
        let connected_peers = Arc::new(AtomicU32::new(0));
        inbound_mapping.push((topic.clone(), inbound_tx, connected_peers.clone()));

        handles_ret.push(GossipHandle {
            connected_peers,
            topic,
            tx_to_outbound: tx_to_outbound.clone(),
            rx_from_inbound: Arc::new(Mutex::new(inbound_rx)),
            logger: logger.clone(),
            ecdsa_peer_id_to_libp2p_id: ecdsa_peer_id_to_libp2p_id.clone(),
        })
    }

    swarm
        .listen_on(format!("/ip4/{}/udp/{}/quic-v1", config.bind_ip, config.bind_port).parse()?)?;
    //swarm.listen_on(format!("/ip4/{}/tcp/{}", config.bind_ip, config.bind_port).parse()?)?;

    // Dial all bootnodes
    for bootnode in &config.bootnodes {
        swarm.dial(
            DialOpts::unknown_peer_id()
                .address(bootnode.clone())
                .build(),
        )?;
    }

    let worker = async move {
        let span = tracing::debug_span!("network_worker");
        let _enter = span.enter();
        let service = NetworkServiceWithoutSwarm {
            logger: &logger,
            inbound_mapping: &inbound_mapping,
            ecdsa_peer_id_to_libp2p_id,
            role_key: &role_key,
            span: tracing::debug_span!(parent: &span, "network_service"),
        };
        loop {
            select! {
                // Setup outbound channel
                Some(msg) = rx_to_outbound.recv() => {
                    service.with_swarm(&mut swarm).handle_intra_node_payload(msg);
                }
                event = swarm.select_next_some() => {
                    service.with_swarm(&mut swarm).handle_swarm_event(event).await;
                }
            }
        }
    };

    let spawn_handle = tokio::task::spawn(worker);
    Ok((handles_ret, spawn_handle))
}

type InboundMapping = (IdentTopic, UnboundedSender<Vec<u8>>, Arc<AtomicU32>);

struct NetworkServiceWithoutSwarm<'a> {
    logger: &'a DebugLogger,
    inbound_mapping: &'a [InboundMapping],
    ecdsa_peer_id_to_libp2p_id: Arc<RwLock<HashMap<ecdsa::Public, PeerId>>>,
    role_key: &'a ecdsa::Public,
    span: tracing::Span,
}

impl<'a> NetworkServiceWithoutSwarm<'a> {
    fn with_swarm(&'a self, swarm: &'a mut libp2p::Swarm<MyBehaviour>) -> NetworkService<'a> {
        NetworkService {
            swarm,
            logger: self.logger,
            inbound_mapping: self.inbound_mapping,
            ecdsa_peer_id_to_libp2p_id: &self.ecdsa_peer_id_to_libp2p_id,
            role_key: self.role_key,
            span: &self.span,
        }
    }
}

struct NetworkService<'a> {
    swarm: &'a mut libp2p::Swarm<MyBehaviour>,
    logger: &'a DebugLogger,
    inbound_mapping: &'a [InboundMapping],
    ecdsa_peer_id_to_libp2p_id: &'a Arc<RwLock<HashMap<ecdsa::Public, PeerId>>>,
    role_key: &'a ecdsa::Public,
    span: &'a tracing::Span,
}

impl<'a> NetworkService<'a> {
    fn handle_intra_node_payload(&mut self, msg: IntraNodePayload) {
        let _enter = self.span.enter();
        match (msg.message_type, msg.payload) {
            (MessageType::Broadcast, GossipOrRequestResponse::Gossip(payload)) => {
                let gossip_message = bincode2::serialize(&payload).expect("Should serialize");
                if let Err(e) = self
                    .swarm
                    .behaviour_mut()
                    .gossipsub
                    .publish(msg.topic, gossip_message)
                {
                    self.logger.error(format!("Publish error: {e:?}"));
                }
            }

            (MessageType::P2P(peer_id), GossipOrRequestResponse::Request(req)) => {
                // Send the outer payload in order to attach the topic to it
                // "Requests are sent using Behaviour::send_request and the responses
                // received as Message::Response via Event::Message."
                self.swarm.behaviour_mut().p2p.send_request(&peer_id, req);
            }
            (MessageType::Broadcast, GossipOrRequestResponse::Request(_)) => {
                self.logger.error("Broadcasting a request is not supported");
            }
            (MessageType::Broadcast, GossipOrRequestResponse::Response(_)) => {
                self.logger
                    .error("Broadcasting a response is not supported");
            }
            (MessageType::P2P(_), GossipOrRequestResponse::Gossip(_)) => {
                self.logger
                    .error("P2P message should be a request or response");
            }
            (MessageType::P2P(_), GossipOrRequestResponse::Response(_)) => {
                // TODO: Send the response to the peer.
            }
        }
    }

    #[tracing::instrument(skip(self, event))]
    async fn handle_p2p(
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
        request_id: request_response::OutboundRequestId,
        message: MyBehaviourResponse,
    ) {
        use MyBehaviourResponse::*;
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
        use MyBehaviourRequest::*;
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

    #[tracing::instrument(skip(self, event))]
    async fn handle_gossip(&mut self, event: gossipsub::Event) {
        use gossipsub::Event::*;
        let with_connected_peers = |topic: &TopicHash, f: fn(&Arc<AtomicU32>)| {
            let maybe_mapping = self
                .inbound_mapping
                .iter()
                .find(|r| r.0.to_string() == topic.to_string());
            match maybe_mapping {
                Some((_, _, connected_peers)) => {
                    f(connected_peers);
                    true
                }
                None => false,
            }
        };
        match event {
            Message {
                propagation_source,
                message_id,
                message,
            } => {
                self.handle_gossip_message(propagation_source, message_id, message)
                    .await;
            }
            Subscribed { peer_id, topic } => {
                let added = with_connected_peers(&topic, |connected_peers| {
                    connected_peers.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                });
                if added {
                    self.logger
                        .trace(format!("{peer_id} subscribed to {topic}",));
                } else {
                    self.logger
                        .error(format!("{peer_id} subscribed to unknown topic: {topic}"));
                }
            }
            Unsubscribed { peer_id, topic } => {
                let removed = with_connected_peers(&topic, |connected_peers| {
                    connected_peers.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                });
                if removed {
                    self.logger
                        .trace(format!("{peer_id} unsubscribed from {topic}",));
                } else {
                    self.logger.error(format!(
                        "{peer_id} unsubscribed from unknown topic: {topic}"
                    ));
                }
            }
            GossipsubNotSupported { peer_id } => {
                self.logger
                    .trace(format!("{peer_id} does not support gossipsub!"));
            }
        }
    }

    #[tracing::instrument(
        skip(self, message),
        fields(
            %message_id,
            %propagation_source,
            source = ?message.source
        )
    )]
    async fn handle_gossip_message(
        &mut self,
        propagation_source: PeerId,
        message_id: gossipsub::MessageId,
        message: gossipsub::Message,
    ) {
        let Some(origin) = message.source else {
            self.logger
                .error("Got message from unknown peer".to_string());
            return;
        };
        self.logger
            .debug(format!("Got message from peer: {origin}",));
        match bincode2::deserialize::<GossipMessage>(&message.data) {
            Ok(GossipMessage { topic, raw_payload }) => {
                if let Some((_, tx, _)) = self
                    .inbound_mapping
                    .iter()
                    .find(|r| r.0.to_string() == topic)
                {
                    if let Err(e) = tx.send(raw_payload) {
                        self.logger
                            .error(format!("Failed to send message to worker: {e}"));
                    }
                } else {
                    self.logger
                        .error(format!("No registered worker for topic: {topic}!"));
                }
            }
            Err(e) => {
                self.logger
                    .error(format!("Failed to deserialize message: {e}"));
            }
        }
    }

    #[tracing::instrument(skip(self, event))]
    async fn handle_mdns_event(&mut self, event: mdns::Event) {
        use mdns::Event::*;
        match event {
            Discovered(list) => {
                for (peer_id, multiaddr) in list {
                    self.logger
                        .debug(format!("discovered a new peer: {peer_id} on {multiaddr}"));
                    self.swarm
                        .behaviour_mut()
                        .gossipsub
                        .add_explicit_peer(&peer_id);
                    if let Err(err) = self.swarm.dial(multiaddr) {
                        self.logger.error(format!("Failed to dial peer: {err}"));
                    }
                }
            }
            Expired(list) => {
                for (peer_id, multiaddr) in list {
                    self.logger.debug(format!(
                        "discover peer has expired: {peer_id} with {multiaddr}"
                    ));
                    self.swarm
                        .behaviour_mut()
                        .gossipsub
                        .remove_explicit_peer(&peer_id);
                }
            }
        }
    }

    #[tracing::instrument(skip(self, event))]
    async fn handle_identify_event(&mut self, event: libp2p::identify::Event) {
        use libp2p::identify::Event::*;
        match event {
            Received { peer_id, info } => {
                // TODO: Verify the peer info, for example the protocol version, agent version, etc.
                let info_lines = vec![
                    format!("Protocol Version: {}", info.protocol_version),
                    format!("Agent Version: {}", info.agent_version),
                    format!("Supported Protocols: {:?}", info.protocols),
                ];
                let info_lines = info_lines.join(", ");
                self.logger.debug(format!(
                    "Received identify event from peer: {peer_id} with info: {info_lines}"
                ));
            }
            Sent { peer_id } => {
                self.logger
                    .trace(format!("Sent identify event to peer: {peer_id}"));
            }
            Pushed { peer_id, info } => {
                let info_lines = vec![
                    format!("Protocol Version: {}", info.protocol_version),
                    format!("Agent Version: {}", info.agent_version),
                    format!("Supported Protocols: {:?}", info.protocols),
                ];
                let info_lines = info_lines.join(", ");
                self.logger.debug(format!(
                    "Pushed identify event to peer: {peer_id} with info: {info_lines}"
                ));
            }
            Error { peer_id, error } => {
                self.logger.error(format!(
                    "Identify error from peer: {peer_id} with error: {error}"
                ));
            }
        }
    }

    #[tracing::instrument(skip(self, event))]
    async fn handle_kadmelia_event(&mut self, event: libp2p::kad::Event) {
        // TODO: Handle kadmelia events
        self.logger.trace(format!("Kadmelia event: {event:?}"));
    }

    #[tracing::instrument(skip(self))]
    async fn handle_connection_established(&mut self, peer_id: PeerId, num_established: u32) {
        self.logger.debug("Connection established");
        // TODO: Sign the handshake message.
        let handshake = MyBehaviourRequest::Handshake {
            ecdsa_public_key: *self.role_key,
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

    #[tracing::instrument(skip(self))]
    async fn handle_connection_closed(
        &mut self,
        peer_id: PeerId,
        num_established: u32,
        cause: Option<libp2p::swarm::ConnectionError>,
    ) {
        self.logger.debug("Connection closed");
        self.swarm
            .behaviour_mut()
            .gossipsub
            .remove_explicit_peer(&peer_id);
    }

    #[tracing::instrument(skip(self))]
    async fn handle_incoming_connection(
        &mut self,
        connection_id: libp2p::swarm::ConnectionId,
        local_addr: libp2p::Multiaddr,
        send_back_addr: libp2p::Multiaddr,
    ) {
        self.logger.debug("Incoming connection");
    }

    #[tracing::instrument(skip(self))]
    async fn handle_outgoing_connection(
        &mut self,
        peer_id: PeerId,
        connection_id: libp2p::swarm::ConnectionId,
    ) {
        self.logger
            .debug(format!("Outgoing connection to peer: {peer_id}",));
    }

    #[tracing::instrument(skip(self, error))]
    async fn handle_incoming_connection_error(
        &mut self,
        connection_id: libp2p::swarm::ConnectionId,
        local_addr: libp2p::Multiaddr,
        send_back_addr: libp2p::Multiaddr,
        error: libp2p::swarm::ListenError,
    ) {
        self.logger
            .error(format!("Incoming connection error: {error}",));
    }

    #[tracing::instrument(skip(self, error))]
    async fn handle_outgoing_connection_error(
        &mut self,
        connection_id: libp2p::swarm::ConnectionId,
        peer_id: Option<PeerId>,
        error: libp2p::swarm::DialError,
    ) {
        self.logger
            .error(format!("Outgoing connection error: {error}",));
    }

    async fn handle_swarm_event(&mut self, event: SwarmEvent<MyBehaviourEvent>) {
        use MyBehaviourEvent::*;
        use SwarmEvent::*;
        let _enter = self.span.enter();
        match event {
            Behaviour(P2p(event)) => {
                self.handle_p2p(event).await;
            }
            Behaviour(Gossipsub(event)) => {
                self.handle_gossip(event).await;
            }
            Behaviour(Mdns(event)) => {
                self.handle_mdns_event(event).await;
            }
            Behaviour(Identify(event)) => {
                self.handle_identify_event(event).await;
            }
            Behaviour(Kadmelia(event)) => {
                self.logger.trace(format!("Kadmelia event: {event:?}"));
            }

            NewListenAddr {
                address,
                listener_id,
            } => {
                self.logger
                    .debug(format!("{listener_id} has a new address: {address}"));
            }
            ConnectionEstablished {
                peer_id,
                num_established,
                ..
            } => {
                self.handle_connection_established(peer_id, num_established.get())
                    .await;
            }
            ConnectionClosed {
                peer_id,
                num_established,
                cause,
                ..
            } => {
                self.handle_connection_closed(peer_id, num_established, cause)
                    .await;
            }
            IncomingConnection {
                connection_id,
                local_addr,
                send_back_addr,
            } => {
                self.handle_incoming_connection(connection_id, local_addr, send_back_addr)
                    .await;
            }
            IncomingConnectionError {
                connection_id,
                local_addr,
                send_back_addr,
                error,
            } => {
                self.handle_incoming_connection_error(
                    connection_id,
                    local_addr,
                    send_back_addr,
                    error,
                )
                .await;
            }
            OutgoingConnectionError {
                connection_id,
                peer_id,
                error,
            } => {
                self.handle_outgoing_connection_error(connection_id, peer_id, error)
                    .await;
            }
            ExpiredListenAddr {
                listener_id,
                address,
            } => {
                self.logger
                    .trace(format!("{listener_id} has an expired address: {address}"));
            }
            ListenerClosed {
                listener_id,
                addresses,
                reason,
            } => {
                self.logger.trace(format!(
                    "{listener_id} on {addresses:?} has been closed: {reason:?}"
                ));
            }
            ListenerError { listener_id, error } => {
                self.logger
                    .error(format!("{listener_id} has an error: {error}"));
            }
            Dialing {
                peer_id,
                connection_id,
            } => {
                self.logger.debug(format!(
                    "Dialing peer: {peer_id:?} with connection_id: {connection_id}"
                ));
            }
            NewExternalAddrCandidate { address } => {
                self.logger
                    .trace(format!("New external address candidate: {address}"));
            }
            ExternalAddrConfirmed { address } => {
                self.logger
                    .trace(format!("External address confirmed: {address}"));
            }
            ExternalAddrExpired { address } => {
                self.logger
                    .trace(format!("External address expired: {address}"));
            }
            NewExternalAddrOfPeer { peer_id, address } => {
                self.logger.trace(format!(
                    "New external address of peer: {peer_id} with address: {address}"
                ));
            }
            unknown => {
                self.logger
                    .warn(format!("Unknown swarm event: {unknown:?}"));
            }
        }
    }
}

#[derive(Clone)]
pub struct GossipHandle {
    topic: IdentTopic,
    tx_to_outbound: UnboundedSender<IntraNodePayload>,
    rx_from_inbound: Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>>>,
    logger: DebugLogger,
    connected_peers: Arc<AtomicU32>,
    ecdsa_peer_id_to_libp2p_id: Arc<RwLock<HashMap<ecdsa::Public, PeerId>>>,
}

impl GossipHandle {
    pub fn connected_peers(&self) -> usize {
        self.connected_peers
            .load(std::sync::atomic::Ordering::Relaxed) as usize
    }

    pub fn topic(&self) -> IdentTopic {
        self.topic.clone()
    }
}

pub struct IntraNodePayload {
    topic: IdentTopic,
    payload: GossipOrRequestResponse,
    message_type: MessageType,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum GossipOrRequestResponse {
    Gossip(GossipMessage),
    Request(MyBehaviourRequest),
    Response(MyBehaviourResponse),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GossipMessage {
    topic: String,
    raw_payload: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum MyBehaviourRequest {
    Handshake { ecdsa_public_key: ecdsa::Public },
    Message { topic: String, raw_payload: Vec<u8> },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum MyBehaviourResponse {
    Handshaked { ecdsa_public_key: ecdsa::Public },
    MessageHandled,
}

enum MessageType {
    Broadcast,
    P2P(PeerId),
}

#[async_trait]
impl Network for GossipHandle {
    async fn next_message(&self) -> Option<<WorkManager as WorkManagerInterface>::ProtocolMessage> {
        let mut lock = self
            .rx_from_inbound
            .try_lock()
            .expect("There should be only a single caller for `next_message`");

        let message = lock.recv().await?;
        match bincode2::deserialize(&message) {
            Ok(message) => Some(message),
            Err(e) => {
                self.logger
                    .error(format!("Failed to deserialize message: {e}"));
                drop(lock);
                self.next_message().await
            }
        }
    }

    async fn send_message(
        &self,
        message: <WorkManager as WorkManagerInterface>::ProtocolMessage,
    ) -> Result<(), gadget_common::Error> {
        let message_type = if let Some(to) = message.to_network_id {
            let libp2p_id = self
                .ecdsa_peer_id_to_libp2p_id
                .read()
                .await
                .get(&to)
                .cloned()
                .ok_or_else(|| gadget_common::Error::NetworkError {
                    err: format!(
                        "No libp2p ID found for ecdsa public key: {to}. No handshake happened?"
                    ),
                })?;

            MessageType::P2P(libp2p_id)
        } else {
            MessageType::Broadcast
        };

        let payload_inner = match message_type {
            MessageType::Broadcast => GossipOrRequestResponse::Gossip(GossipMessage {
                topic: self.topic.to_string(),
                raw_payload: bincode2::serialize(&message).expect("Should serialize"),
            }),
            MessageType::P2P(_) => GossipOrRequestResponse::Request(MyBehaviourRequest::Message {
                topic: self.topic.to_string(),
                raw_payload: bincode2::serialize(&message).expect("Should serialize"),
            }),
        };

        let payload = IntraNodePayload {
            topic: self.topic.clone(),
            payload: payload_inner,
            message_type,
        };

        self.tx_to_outbound
            .send(payload)
            .map_err(|e| gadget_common::Error::NetworkError {
                err: format!("Failed to send intra-node payload: {e}"),
            })
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{KeystoreConfig, ShellConfig, SubxtConfig};
    use crate::network::setup_network;
    use crate::shell::wait_for_connection_to_bootnodes;
    use gadget_common::prelude::{DebugLogger, GadgetProtocolMessage, Network, WorkManager};
    use gadget_core::job_manager::WorkManagerInterface;
    use sp_core::ecdsa;

    #[tokio::test]
    async fn test_gossip_network() {
        color_eyre::install().unwrap();
        test_utils::setup_log();
        const N_PEERS: usize = 3;
        let networks = vec!["/test-network"];
        let mut all_handles = Vec::new();
        for x in 0..N_PEERS {
            let identity = libp2p::identity::Keypair::generate_ed25519();

            let logger = DebugLogger {
                peer_id: identity.public().to_peer_id().to_string(),
            };

            let (bind_port, bootnodes) = if x == 0 {
                (
                    30555,
                    vec![
                        "/ip4/0.0.0.0/tcp/30556".parse().unwrap(),
                        "/ip4/0.0.0.0/tcp/30557".parse().unwrap(),
                    ],
                )
            } else if x == 1 {
                (
                    30556,
                    vec![
                        "/ip4/0.0.0.0/tcp/30555".parse().unwrap(),
                        "/ip4/0.0.0.0/tcp/30557".parse().unwrap(),
                    ],
                )
            } else if x == 2 {
                (
                    30557,
                    vec![
                        "/ip4/0.0.0.0/tcp/30555".parse().unwrap(),
                        "/ip4/0.0.0.0/tcp/30556".parse().unwrap(),
                    ],
                )
            } else {
                panic!("Invalid peer index");
            };

            let shell_config = ShellConfig {
                keystore: KeystoreConfig::InMemory,
                subxt: SubxtConfig {
                    endpoint: url::Url::from_directory_path("/").unwrap(),
                },
                bind_ip: "0.0.0.0".to_string(),
                bind_port,
                bootnodes,
                node_key: [0u8; 32],
            };

            let role_key = get_dummy_role_key_from_index(x);

            let (handles, _) = setup_network(
                identity,
                &shell_config,
                logger.clone(),
                networks.clone(),
                role_key,
            )
            .await
            .unwrap();
            all_handles.push((handles, shell_config, logger));
        }

        for (handles, shell_config, logger) in &all_handles {
            wait_for_connection_to_bootnodes(shell_config, handles, logger)
                .await
                .unwrap();
        }

        /*
           We must test the following:
           * Broadcast send
           * Broadcast receive
           * P2P send
           * P2P receive
        */

        // Now, send broadcast messages through each topic
        for _network in &networks {
            for (handles, _, _) in &all_handles {
                for handle in handles {
                    handle
                        .send_message(dummy_message_broadcast(b"Hello, world".to_vec()))
                        .await
                        .unwrap();
                }
            }
        }

        // Next, receive these broadcasted messages
        for _network in &networks {
            for (handles, _, logger) in &all_handles {
                logger.debug("Waiting to receive broadcast messages ...");
                for handle in handles {
                    let message = handle.next_message().await.unwrap();
                    assert_eq!(message.payload, b"Hello, world");
                    assert_eq!(message.to_network_id, None);
                }
            }
        }

        let send_idxs = [0, 1, 2];
        // Next, send P2P messages: everybody sends a message to everybody
        for _network in networks.iter() {
            for (handles, _, _) in all_handles.iter() {
                for (my_idx, handle) in handles.iter().enumerate() {
                    let send_idxs = send_idxs
                        .iter()
                        .filter(|&&idx| idx != my_idx)
                        .cloned()
                        .collect::<Vec<_>>();
                    for i in send_idxs {
                        handle
                            .send_message(dummy_message_p2p(b"Hello, world".to_vec(), i))
                            .await
                            .unwrap();
                    }
                }
            }
        }

        // Finally, receive P2P messages: everybody should receive a message from everybody else
        for _network in networks.iter() {
            for (handles, _, logger) in all_handles.iter() {
                logger.debug("Waiting to receive P2P messages ...");
                for (my_idx, handle) in handles.iter().enumerate() {
                    // Each party should receive two messages
                    for _ in 0..2 {
                        let message = handle.next_message().await.unwrap();
                        assert_eq!(message.payload, b"Hello, world");
                        assert_eq!(
                            message.to_network_id,
                            Some(get_dummy_role_key_from_index(my_idx))
                        );
                    }
                }
            }
        }
    }

    fn dummy_message_broadcast(
        input: Vec<u8>,
    ) -> <WorkManager as WorkManagerInterface>::ProtocolMessage {
        dummy_message_inner(input, None)
    }

    fn dummy_message_p2p(
        input: Vec<u8>,
        to_idx: usize,
    ) -> <WorkManager as WorkManagerInterface>::ProtocolMessage {
        let dummy_role_key = get_dummy_role_key_from_index(to_idx);
        dummy_message_inner(input, Some(dummy_role_key))
    }

    fn dummy_message_inner(
        input: Vec<u8>,
        to_network_id: Option<ecdsa::Public>,
    ) -> <WorkManager as WorkManagerInterface>::ProtocolMessage {
        GadgetProtocolMessage {
            associated_block_id: 0,
            associated_session_id: 0,
            associated_retry_id: 0,
            task_hash: [0u8; 32],
            from: 0,
            to: None,
            payload: input,
            from_network_id: None,
            to_network_id,
        }
    }

    fn get_dummy_role_key_from_index(index: usize) -> ecdsa::Public {
        ecdsa::Public::from_raw([index as u8; 33])
    }
}
