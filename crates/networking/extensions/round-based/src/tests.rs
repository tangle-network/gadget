use gadget_networking::{KeyType, NetworkConfig, NetworkService};
use libp2p::Multiaddr;
use round_based::ProtocolMessage;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Serialize, Deserialize, Clone, ProtocolMessage)]
enum Msg {
    Round1(Round1Msg),
    Round2(Round2Msg),
    Round3(Round3Msg),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Round1Msg {
    pub power: u16,
    pub hitpoints: u16,
    pub armor: u16,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Round2Msg {
    pub x: u16,
    pub y: u16,
    pub z: u16,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Round3Msg {
    rotation: u16,
    velocity: (u16, u16, u16),
}

const TOPIC: &str = "/gadget/test/1.0.0";

fn node() -> NetworkService {
    let local_key = libp2p::identity::Keypair::generate_ed25519();
    let listen_addr: Multiaddr = "/ip4/127.0.0.1/tcp/0".parse().unwrap();

    let instance_key_pair = gadget_networking::Curve::generate_with_seed(None).unwrap();

    let config = NetworkConfig {
        network_name: TOPIC.to_string(),
        instance_id: String::from("0"),
        instance_key_pair,
        local_key,
        listen_addr,
        target_peer_count: 0,
        bootstrap_peers: vec![],
        enable_mdns: true,
        enable_kademlia: true,
    };

    NetworkService::new(config, HashSet::default()).unwrap()
}

#[tokio::test]
async fn round_based() {
    let node = node();
}
