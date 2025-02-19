use super::TestNode;
use super::{init_tracing, wait_for_peer_discovery};
use super::{wait_for_peer_info, NetworkServiceHandleExt};
use std::collections::HashSet;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_peer_handshake() {
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

    handle1.dial(&handle2);

    let identify_timeout = Duration::from_secs(20);
    wait_for_peer_info(&handle1, &handle2, identify_timeout).await;

    let node2_peer = handle2.local_peer_id;
    assert!(
        handle1.peer_manager.is_peer_verified(&node2_peer),
        "Node2 was not verified, handshake failed"
    );
}
