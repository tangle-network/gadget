use crate::service::NetworkMessage;
use crate::{
    key_types::InstanceMsgPublicKey, service_handle::NetworkServiceHandle, InstanceMsgKeyPair,
    NetworkConfig, NetworkService,
};
use gadget_crypto::{sp_core::SpEcdsa, KeyType};
use libp2p::{
    identity::{self, Keypair},
    Multiaddr, PeerId,
};
use std::{collections::HashSet, time::Duration};
use tokio::time::timeout;
use tracing::info;

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
/// Test node configuration for network tests
pub struct TestNode {
    pub service: Option<NetworkService>,
    pub peer_id: PeerId,
    pub listen_addr: Option<Multiaddr>,
    pub instance_key_pair: InstanceMsgKeyPair,
    pub local_key: Keypair,
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
            instance_key_pair: instance_key_pair.clone(),
            local_key: local_key.clone(),
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
            listen_addr: None,
            instance_key_pair,
            local_key,
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
    handles: &[&NetworkServiceHandle],
    timeout: Duration,
) -> Result<(), &'static str> {
    info!("Waiting for peer discovery...");

    wait_for_condition(timeout, || {
        for (i, handle1) in handles.iter().enumerate() {
            for (j, handle2) in handles.iter().enumerate() {
                if i != j
                    && !handle1
                        .peers()
                        .iter()
                        .any(|id| *id == handle2.local_peer_id)
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
    handle1: &NetworkServiceHandle,
    handle2: &NetworkServiceHandle,
    timeout: Duration,
) {
    info!("Waiting for identify info...");

    match tokio::time::timeout(timeout, async {
        loop {
            let peer_info1 = handle1.peer_info(&handle2.local_peer_id);
            let peer_info2 = handle2.peer_info(&handle1.local_peer_id);

            if let Some(peer_info) = peer_info1 {
                if peer_info.identify_info.is_some() {
                    // Also verify reverse direction
                    if let Some(peer_info) = peer_info2 {
                        if peer_info.identify_info.is_some() {
                            info!("Identify info populated in both directions");
                            break;
                        }
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    {
        Ok(_) => info!("Peer info updated successfully in both directions"),
        Err(_) => panic!("Peer info update timed out"),
    }
}
