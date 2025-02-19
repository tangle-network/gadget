use std::{collections::HashSet, mem, time::Duration};

use gadget_crypto::{sp_core::SpEcdsa, KeyType};
use libp2p::{identity, Multiaddr, PeerId};
use tokio::time::timeout;
use tracing::info;

use crate::{
    key_types::InstanceMsgPublicKey, service_handle::NetworkServiceHandle, NetworkConfig,
    NetworkService,
};

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
        bootstrap_peers: Vec<(PeerId, Multiaddr)>,
    ) -> Self {
        let local_key = identity::Keypair::generate_ed25519();
        let peer_id = local_key.public().to_peer_id();

        let listen_addr: Multiaddr = format!("/ip4/127.0.0.1/tcp/0").parse().unwrap();
        info!(
            "Creating test node {} with TCP address: {}",
            peer_id, listen_addr
        );

        let instance_secret_key = SpEcdsa::generate_with_seed(None).unwrap();
        let instance_public_key = instance_secret_key.public();

        let config = NetworkConfig {
            network_name: network_name.to_string(),
            instance_id: instance_id.to_string(),
            instance_secret_key,
            instance_public_key,
            local_key,
            listen_addr: listen_addr.clone(),
            target_peer_count: 10,
            bootstrap_peers,
            enable_mdns: true,
            enable_kademlia: true,
        };

        let (allowed_keys_tx, allowed_keys_rx) = crossbeam_channel::unbounded();
        allowed_keys_tx.send(allowed_keys.clone()).unwrap();
        let service = NetworkService::new(config, allowed_keys, allowed_keys_rx)
            .await
            .expect("Failed to create network service");

        Self {
            service: Some(service),
            peer_id,
            listen_addr: Some(listen_addr),
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
                if let Some(addr) = handle.get_listen_addr().await {
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
