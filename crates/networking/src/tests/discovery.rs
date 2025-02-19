use crate::service::NetworkMessage;
use crate::test_helpers::TestNode;
use std::{collections::HashSet, time::Duration};
use tokio::time::timeout;
use tracing::{debug, info};
use tracing_subscriber::{fmt, EnvFilter};

fn init_tracing() {
    let _ = fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .try_init();
}

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

    // Wait for peer discovery (with timeout)
    let discovery_timeout = Duration::from_secs(10);
    match timeout(discovery_timeout, async {
        loop {
            let peers1 = handle1.peers();
            let peers2 = handle2.peers();

            if peers1.contains(&node2.peer_id) && peers2.contains(&node1.peer_id) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    {
        Ok(_) => println!("Peers discovered each other successfully"),
        Err(_) => panic!("Peer discovery timed out"),
    }
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

    info!("Waiting for peer discovery...");
    // First wait for basic peer discovery (they see each other)
    let discovery_timeout = Duration::from_secs(20);
    timeout(discovery_timeout, async {
        loop {
            let peers1 = handle1.peers();
            let peers2 = handle2.peers();

            debug!("Node1 peers: {:?}, Node2 peers: {:?}", peers1, peers2);

            if peers1.contains(&node2.peer_id) && peers2.contains(&node1.peer_id) {
                info!("Basic peer discovery successful");
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("Basic peer discovery timed out");

    let node2_listen_addr = node2
        .listen_addr
        .expect("Node2 did not return a listening address");

    handle1
        .send_network_message(NetworkMessage::Dial(node2_listen_addr.clone()))
        .expect("Node1 failed to dial Node2");

    info!("Waiting for identify info...");
    // Now wait for identify info to be populated
    let identify_timeout = Duration::from_secs(20);
    match timeout(identify_timeout, async {
        loop {
            let peer_info1 = handle1.peer_info(&node2.peer_id);
            let peer_info2 = handle2.peer_info(&node1.peer_id);

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
