use super::init_tracing;
use super::TestNode;
use super::{wait_for_peer_discovery, wait_for_peer_info};
use crate::service::NetworkMessage;
use std::{collections::HashSet, time::Duration};
use tokio::time::timeout;
use tracing::{debug, info};
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::test]
async fn test_peer_discovery_mdns() {
    init_tracing();

    let network_name = "test-network";
    let instance_id = "test-instance";
    let allowed_keys = HashSet::new();

    // Create two nodes
    let mut node1 = TestNode::new(network_name, instance_id, allowed_keys.clone(), vec![]).await;
    let mut node2 = TestNode::new(network_name, instance_id, allowed_keys, vec![]).await;

    // Start both nodes and wait for them to be listening
    let handle1 = node1.start().await.expect("Failed to start node1");
    let handle2 = node2.start().await.expect("Failed to start node2");

    // First wait for basic peer discovery (they see each other)
    let discovery_timeout = Duration::from_secs(20);
    wait_for_peer_discovery(&[&handle1, &handle2], discovery_timeout)
        .await
        .expect("Basic peer discovery timed out");
}

#[tokio::test]
async fn test_peer_discovery_kademlia() {
    init_tracing();

    let network_name = "test-network";
    let instance_id = "test-instance";
    let allowed_keys = HashSet::new();

    // Create the first node (bootstrap node)
    let mut node1 = TestNode::new(network_name, instance_id, allowed_keys.clone(), vec![]).await;

    // Start node1 and get its listening address
    let handle1 = node1.start().await.expect("Failed to start node1");
    let node1_addr = node1.get_listen_addr().expect("Node1 should be listening");

    // Create two more nodes that will bootstrap from node1
    let bootstrap_peers = vec![node1_addr.clone()];
    let mut node2 = TestNode::new(
        network_name,
        instance_id,
        allowed_keys.clone(),
        bootstrap_peers.clone(),
    )
    .await;
    let mut node3 = TestNode::new(network_name, instance_id, allowed_keys, bootstrap_peers).await;

    // Start the remaining nodes
    let handle2 = node2.start().await.expect("Failed to start node2");
    let handle3 = node3.start().await.expect("Failed to start node3");

    // Wait for peer discovery through Kademlia DHT
    let discovery_timeout = Duration::from_secs(20);
    match timeout(discovery_timeout, async {
        loop {
            let peers1 = handle1.peers();
            let peers2 = handle2.peers();
            let peers3 = handle3.peers();

            if peers1.contains(&node2.peer_id)
                && peers1.contains(&node3.peer_id)
                && peers2.contains(&node1.peer_id)
                && peers2.contains(&node3.peer_id)
                && peers3.contains(&node1.peer_id)
                && peers3.contains(&node2.peer_id)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    {
        Ok(_) => println!("All peers discovered each other through Kademlia"),
        Err(_) => panic!("Kademlia peer discovery timed out"),
    }
}

#[tokio::test]
async fn test_peer_info_updates() {
    init_tracing();

    let network_name = "test-network";
    let instance_id = "test-instance";
    let allowed_keys = HashSet::new();

    info!("Creating test nodes...");
    // Create two nodes
    let mut node1 = TestNode::new(network_name, instance_id, allowed_keys.clone(), vec![]).await;
    let mut node2 = TestNode::new(network_name, instance_id, allowed_keys, vec![]).await;

    info!("Starting nodes...");
    // Start both nodes
    let mut handle1 = node1.start().await.expect("Failed to start node1");
    let handle2 = node2.start().await.expect("Failed to start node2");

    // First wait for basic peer discovery (they see each other)
    let discovery_timeout = Duration::from_secs(20);
    wait_for_peer_discovery(&[&handle1, &handle2], discovery_timeout)
        .await
        .expect("Basic peer discovery timed out");

    // Now wait for identify info to be populated
    let identify_timeout = Duration::from_secs(20);
    wait_for_peer_info(&handle1, &handle2, identify_timeout).await;
}
