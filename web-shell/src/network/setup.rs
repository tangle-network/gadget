use crate::config::ShellConfig;
use crate::network::gossip::{
    GossipHandle, IntraNodePayload, MyBehaviour, NetworkServiceWithoutSwarm, MAX_MESSAGE_SIZE,
};
use crate::shell::{AGENT_VERSION, CLIENT_VERSION};
use futures::StreamExt;
use gadget_common::config::DebugLogger;
use libp2p::gossipsub::IdentTopic;
use libp2p::kad::store::MemoryStore;
use libp2p::swarm::dial_opts::DialOpts;
// use libp2p::{gossipsub, mdns, request_response, StreamProtocol};
use libp2p::{gossipsub, request_response, StreamProtocol};
use sp_core::{ecdsa, Pair};
use std::collections::HashMap;
use std::error::Error;
use std::io;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::time::Duration;
use std::str::FromStr;
use std::net::{Ipv4Addr, SocketAddr};
use gadget_io::tokio::select;
use gadget_io::tokio::sync::{Mutex, RwLock};
use gadget_io::tokio::task::JoinHandle;

use js_sys::Date;
// use libp2p::core::Multiaddr;
use libp2p::ping;
use libp2p::swarm::SwarmEvent;
use libp2p_webrtc_websys as webrtc_websys;
use wasm_bindgen::prelude::*;
use web_sys::{Document, HtmlElement};
use libp2p_webtransport_websys as webtransport_websys;

use k256::ecdsa::{SigningKey as SecretKey, VerifyingKey};
// use p256::ecdsa::{SigningKey as SecretKey, VerifyingKey};

use libp2p::{
    multiaddr::{Multiaddr, Protocol},
    identity::PeerId,
};

use matchbox_socket::WebRtcSocket;
use matchbox_socket::MessageLoopFuture;
use matchbox_socket::SingleChannel;

use crate::log;

// pub async fn setup_matchbox_network() {
//     //wasm_bindgen_futures::spawn_local(matchbox_listener());
//     let (mut socket, loop_fut) = matchbox_listener().await;
//
// }

pub async fn matchbox_listener(
    // signal_server: SocketAddr,
    // signal_port: u16,
) {
    log(&format!("Starting Matchbox Listener"));
    let (mut socket, loop_fut) = WebRtcSocket::new_reliable("ws://localhost:3536/"); // Signaling Server Address

    let loop_fut = loop_fut.fuse();
    futures::pin_mut!(loop_fut);

    let timeout = Delay::new(Duration::from_millis(100));
    futures::pin_mut!(timeout);

    loop {

        for (peer, state) in socket.update_peers() {
            match state {
                PeerState::Connected => {
                    // TODO: Handle any new peers
                }
                PeerState::Disconnected => {
                    // TODO: Handle any disconnected peers
                }
            }
        }

        // Accept any messages incoming
        for (peer, packet) in socket.receive() {
            // TODO: Handle incoming packets
        }

        select! {
            // Restart this loop every 100ms
            _ = (&mut timeout).fuse() => {
                timeout.reset(Duration::from_millis(100));
            }

            // Break if the message loop ends via Disconnect/Closure
            _ = &mut loop_fut => {
                // TODO: Gracefully close
                break;
            }
        }
    }
}

pub async fn setup_libp2p_network(
    identity: libp2p::identity::Keypair,
    config: &ShellConfig,
    logger: DebugLogger,
    networks: Vec<&'static str>,
    role_key: ecdsa::Pair,
) -> Result<(HashMap<&'static str, GossipHandle>, JoinHandle<()>), Box<dyn Error>> {
    // Setup both QUIC (UDP) and TCP transports the increase the chances of NAT traversal
    log(&format!("Setup Libp2p Network - About to build Swarm"));

    // let mut swarm = libp2p::SwarmBuilder::with_existing_identity(identity.clone())
    //     .with_wasm_bindgen()
    //     .with_other_transport(|key| {
    //         webrtc_websys::Transport::new(webrtc_websys::Config::new(&key))
    //         // webtransport_websys::Transport::new(webtransport_websys::Config::new(&key))
    //     })?
    //     .with_behaviour(|_| ping::Behaviour::new(ping::Config::new()))?
    //     .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
    //     .build();

    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(identity)
        // .with_wasm_bindgen()
        // .with_other_transport(|key| {
        //     webtransport_websys::Transport::new(webtransport_websys::Config::new(&key))
        // })?
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default()
                .port_reuse(true)
                .nodelay(true), // Allow port reuse for TCP-hole punching
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )?
        .with_quic_config(|mut config| {
            config.handshake_timeout = Duration::from_secs(30);
            config
        })
        .with_dns()?
        .with_relay_client(libp2p::noise::Config::new, libp2p::yamux::Config::default)?
        .with_behaviour(|key, relay_client| {
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
            // let mdns =
            //     mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())?;

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

            // Setup dcutr for upgrading existing connections to use relay against the bootnodes when necessary
            // This also provided hole-punching capabilities to attempt to seek a direct connection, and fallback to relaying
            // otherwise.
            // dcutr = direct connection upgrade through relay
            let dcutr = libp2p::dcutr::Behaviour::new(key.public().to_peer_id());

            // Setup relay for using the dcutr-upgraded connections to relay messages for other peers when required
            let relay_config = libp2p::relay::Config::default();
            let relay = libp2p::relay::Behaviour::new(key.public().to_peer_id(), relay_config);

            // Setup ping for liveness checks between connections
            let ping = libp2p::ping::Behaviour::new(libp2p::ping::Config::default());

            Ok(MyBehaviour {
                gossipsub,
                // mdns,
                p2p,
                identify,
                kadmelia,
                dcutr,
                relay,
                relay_client,
                ping,
            })
        })?
        // .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();
    log(&format!("Setup Libp2p Network - Swarm Built"));

    // Subscribe to all networks
    let mut inbound_mapping = Vec::new();
    let (tx_to_outbound, mut rx_to_outbound) =
        gadget_io::tokio::sync::mpsc::unbounded_channel::<IntraNodePayload>();
    let ecdsa_peer_id_to_libp2p_id = Arc::new(RwLock::new(HashMap::new()));
    let mut handles_ret = HashMap::with_capacity(networks.len());
    log(&format!("Setup Libp2p Network - Starting Network Loop"));
    for network in networks {
        let topic = IdentTopic::new(network);
        // swarm.behaviour_mut().gossipsub.subscribe(&topic)?;
        let (inbound_tx, inbound_rx) = gadget_io::tokio::sync::mpsc::unbounded_channel();
        let connected_peers = Arc::new(AtomicU32::new(0));
        inbound_mapping.push((topic.clone(), inbound_tx, connected_peers.clone()));

        handles_ret.insert(
            network,
            GossipHandle {
                connected_peers,
                topic,
                tx_to_outbound: tx_to_outbound.clone(),
                rx_from_inbound: Arc::new(Mutex::new(inbound_rx)),
                logger: logger.clone(),
                ecdsa_peer_id_to_libp2p_id: ecdsa_peer_id_to_libp2p_id.clone(),
            },
        );
    }
    log(&format!("Setup Libp2p Network - Network Loop Completed"));

    // log(&format!("/ip4/{}/udp/{}/webrtc/p2p/{:?}", config.bind_ip, config.bind_port, peer_id));
    // let webrtc_multiaddr = format!("/ip4/{}/udp/{}/webrtc-direct/p2p/{:?}", config.bind_ip, config.bind_port, "QmYyQSo1c1Ym7orWxLYvCrM2EmxFTANf8wXmmE7DWjhx5N").parse()?;
    // let webrtc_multiaddr = format!("/ip4/{}/udp/{}/quic/webtransport/certhash/QmYyQSo1c1Ym7orWxLYvCrM2EmxFTANf8wXmmE7DWjhx5N", config.bind_ip, config.bind_port).parse()?;
    // log(&format!("Setup Libp2p Network - Webrtc Multiaddr: {}", webrtc_multiaddr));
    // let webrtc_multiaddr = Multiaddr::from(config.bind_ip)
    //     .with(Protocol::Udp(config.bind_port))
    //     .with(Protocol::WebRTCDirect);
        // .with(Protocol::P2p(PeerId::from_str("QmYyQSo1c1Ym7orWxLYvCrM2EmxFTANf8wXmmE7DWjhx5N")?));
    // let webtransport_multiaddr = Multiaddr::from(config.bind_ip)
    //     .with(Protocol::Udp(config.bind_port))
    //     .with(Protocol::QuicV1)
    //     .with(Protocol::WebTransport)
    //     .with(Protocol::Certhash(multihash::Multihash::from_bytes("b2uaraocy6yrdblb4sfptaddgimjmmpy".as_bytes())?));

    // swarm.listen_on(webtransport_multiaddr)?;
    // let webtransport_multiaddr = format!("/ip4/127.0.0.1/udp/8081/quic/webtransport/certhash/b2uaraocy6yrdblb4sfptaddgimjmmpy").parse()?;
    let webtransport_multiaddr = format!("/ip4/127.0.0.1/udp/8081/webrtc/certhash/b2uaraocy6yrdblb4sfptaddgimjmmpy/p2p/{:?}", identity.public().to_peer_id()).parse()?;
    swarm.listen_on(webtransport_multiaddr)?;

    log(&format!("Setup Libp2p Network - Swarm Listening for Webrtc"));
    // swarm
    //     .listen_on(format!("/ip4/{}/udp/{}/quic-v1", config.bind_ip, config.bind_port).parse()?)?;
    // swarm.listen_on(format!("/ip4/{}/tcp/{}", config.bind_ip, config.bind_port).parse()?)?;

    log(&format!("Setup Libp2p Network - Dialing Bootnodes"));
    // Dial all bootnodes
    for bootnode in &config.bootnodes {
        swarm.dial(
            DialOpts::unknown_peer_id()
                .address(bootnode.clone())
                .build(),
        )?;
    }
    log(&format!("Setup Libp2p Network - Bootnodes Dialed"));

    let worker = async move {
        let span = tracing::debug_span!("network_worker");
        // let _enter = span.enter();
        span.enter();
        let service = NetworkServiceWithoutSwarm {
            logger: &logger,
            inbound_mapping: &inbound_mapping,
            ecdsa_peer_id_to_libp2p_id,
            role_key: &role_key,
            span: tracing::debug_span!(parent: &span, "network_service"),
        };
        // loop {
        //     select! {
        //         // Setup outbound channel
        //         Some(msg) = rx_to_outbound.recv() => {
        //             service.with_swarm(&mut swarm).handle_intra_node_payload(msg);
        //         }
        //         event = swarm.select_next_some() => {
        //             service.with_swarm(&mut swarm).handle_swarm_event(event).await;
        //         }
        //     }
        // }
    };

    log(&format!("Setup Libp2p Network - Spawning Worker"));
    let spawn_handle = gadget_io::tokio::task::spawn(worker);
    Ok((handles_ret, spawn_handle))
}
