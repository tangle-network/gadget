use crate::service::NetworkMessage;
use std::{collections::HashSet, mem, time::Duration};

use gadget_crypto::{sp_core::SpEcdsa, KeyType};
use libp2p::{identity, Multiaddr, PeerId};
use tokio::time::timeout;
use tracing::info;

use crate::{
    key_types::InstanceMsgPublicKey, service_handle::NetworkServiceHandle, NetworkConfig,
    NetworkService,
};

mod discovery;
mod handshake;

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .try_init();
}

/// Test node configuration for network tests
pub struct TestNode {
    pub service: Option<NetworkService>,
    pub peer_id: PeerId,
    pub listen_addr: Option<Multiaddr>,
}

impl TestNode {
    pub async fn new(
        network_name: &str,
        instance_id: &str,
        allowed_keys: HashSet<InstanceMsgPublicKey>,
        bootstrap_peers: Vec<Multiaddr>,
    ) -> Self {
        let local_key = identity::Keypair::generate_ed25519();
        let peer_id = local_key.public().to_peer_id();

        let listen_addr: Multiaddr = "/ip4/127.0.0.1/tcp/0".parse().unwrap();
        info!("Creating test node {peer_id} with TCP address: {listen_addr}");

        let instance_key_pair = SpEcdsa::generate_with_seed(None).unwrap();

        let config = NetworkConfig {
            network_name: network_name.to_string(),
            instance_id: instance_id.to_string(),
            instance_key_pair,
            local_key,
            listen_addr: listen_addr.clone(),
            target_peer_count: 10,
            bootstrap_peers,
            enable_mdns: true,
            enable_kademlia: true,
        };

        let (allowed_keys_tx, allowed_keys_rx) = crossbeam_channel::unbounded();
        allowed_keys_tx.send(allowed_keys.clone()).unwrap();
        let service = NetworkService::new(config, allowed_keys) // TODO: allowed_keys_rx
            .expect("Failed to create network service");

        Self {
            service: Some(service),
            peer_id,
            listen_addr: None, // To be set later
        }
    }

    /// Start the node
    pub async fn start(&mut self) -> Result<NetworkServiceHandle, &'static str> {
        // Take ownership of the service
        let service = self.service.take().ok_or("Service already started")?;
        let handle = service.start();

        // Wait for the actual listening address
        let timeout_duration = Duration::from_secs(5);
        match timeout(timeout_duration, async {
            while self.listen_addr.is_none() {
                if let Some(addr) = handle.get_listen_addr() {
                    info!("Node {} listening on {}", self.peer_id, addr);
                    self.listen_addr = Some(addr);
                    break;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
            .await
        {
            Ok(_) => Ok(handle),
            Err(_) => Err("Timeout waiting for node to start listening"),
        }
    }

    /// Get the actual listening address
    pub fn get_listen_addr(&self) -> Option<Multiaddr> {
        self.listen_addr.clone()
    }
}

/// Wait for a condition with timeout
pub async fn wait_for_condition<F>(timeout: Duration, mut condition: F) -> Result<(), &'static str>
where
    F: FnMut() -> bool,
{
    let start = std::time::Instant::now();
    while !condition() {
        if start.elapsed() > timeout {
            return Err("Timeout waiting for condition");
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Ok(())
}

/// Wait for peers to discover each other
pub async fn wait_for_peer_discovery(
    nodes: &[&TestNode],
    timeout: Duration,
) -> Result<(), &'static str> {
    wait_for_condition(timeout, || {
        for (i, node1) in nodes.iter().enumerate() {
            for (j, node2) in nodes.iter().enumerate() {
                if i != j
                    && !node1
                    .service
                    .as_ref()
                    .map(|s| s.peer_manager.get_peers())
                    .unwrap_or_default()
                    .contains_key(&node2.peer_id)
                {
                    return false;
                }
            }
        }
        true
    })
        .await
}

/// Wait for peer info to be updated
pub async fn wait_for_peer_info(
    node: &TestNode,
    peer_id: &PeerId,
    timeout: Duration,
) -> Result<(), &'static str> {
    wait_for_condition(timeout, || {
        node.service
            .as_ref()
            .map(|s| s.peer_manager.get_peer_info(peer_id))
            .unwrap_or(None)
            .map_or(false, |info| info.identify_info.is_some())
    })
        .await
}

trait NetworkServiceHandleExt {
    fn dial(&self, other: &NetworkServiceHandle);
}

impl NetworkServiceHandleExt for NetworkServiceHandle {
    fn dial(&self, other: &NetworkServiceHandle) {
        let listen_addr = other.get_listen_addr()
            .expect("Node2 did not return a listening address");

        self
            .send_network_message(NetworkMessage::Dial(listen_addr.clone()))
            .expect("Node1 failed to dial Node2");
    }
}