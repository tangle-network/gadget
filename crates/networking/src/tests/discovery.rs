use super::TestNode;
use super::{create_whitelisted_nodes, init_tracing};
use super::{wait_for_peer_discovery, wait_for_peer_info};
use std::{collections::HashSet, time::Duration};
use tokio::time::timeout;
use tracing::info;

#[tokio::test]
async fn test_peer_discovery_mdns() {
    init_tracing();

    let network_name = "test-network";
    let instance_id = "test-instance";
    let allowed_keys = HashSet::new();

    // Create two nodes
    let mut node1 = TestNode::new(network_name, instance_id, allowed_keys.clone(), vec![]);
    let mut node2 = TestNode::new(network_name, instance_id, allowed_keys, vec![]);

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
    let mut node1 = TestNode::new(network_name, instance_id, allowed_keys.clone(), vec![]);

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
    );
    let mut node3 = TestNode::new(network_name, instance_id, allowed_keys, bootstrap_peers);

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
        Ok(()) => println!("All peers discovered each other through Kademlia"),
        Err(e) => panic!("Kademlia peer discovery timed out: {e}"),
    }
}

#[tokio::test]
async fn test_peer_info_updates() {
    init_tracing();

    info!("Creating test nodes...");
    // Create nodes with whitelisted keys
    let mut nodes = create_whitelisted_nodes(2).await;
    let mut node2 = nodes.pop().unwrap();
    let mut node1 = nodes.pop().unwrap();

    info!("Starting node1...");
    let handle1 = node1.start().await.expect("Failed to start node1");
    info!("Node1 started successfully");

    info!("Starting node2...");
    let handle2 = node2.start().await.expect("Failed to start node2");
    info!("Node2 started successfully");

    info!("Both nodes started, waiting for peer discovery...");

    // First wait for basic peer discovery (they see each other)
    let discovery_timeout = Duration::from_secs(30); // Increased timeout
    match wait_for_peer_discovery(&[&handle1, &handle2], discovery_timeout).await {
        Ok(()) => info!("Peer discovery successful"),
        Err(e) => {
            // Log peer states before failing
            info!("Node1 peers: {:?}", handle1.peers());
            info!("Node2 peers: {:?}", handle2.peers());
            panic!("Peer discovery failed: {}", e);
        }
    }

    info!("Peers discovered each other, waiting for identify info...");

    // Now wait for identify info to be populated
    let identify_timeout = Duration::from_secs(30); // Increased timeout
    wait_for_peer_info(&handle1, &handle2, identify_timeout).await;

    info!("Test completed successfully - both nodes have identify info");

    // Log final state
    if let Some(info) = handle1.peer_info(&handle2.local_peer_id) {
        info!("Node1's info about Node2: {:?}", info);
    }
    if let Some(info) = handle2.peer_info(&handle1.local_peer_id) {
        info!("Node2's info about Node1: {:?}", info);
    }
}
