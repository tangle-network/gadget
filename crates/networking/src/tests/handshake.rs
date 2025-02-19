use super::{
    init_tracing, wait_for_peer_discovery, wait_for_peer_info, NetworkServiceHandleExt, TestNode,
};
use crate::{
    blueprint_protocol::{InstanceMessageRequest, InstanceMessageResponse},
    key_types::{Curve, InstanceMsgKeyPair, InstanceMsgPublicKey},
    service::NetworkMessage,
    service_handle::NetworkServiceHandle,
};
use gadget_crypto::KeyType;
use std::{collections::HashSet, time::Duration};
use tokio::time::timeout;
use tracing::{debug, info};
use tracing_subscriber::{fmt, EnvFilter};

const TEST_TIMEOUT: Duration = Duration::from_secs(5);

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

#[tokio::test]
async fn test_automatic_handshake() {
    init_tracing();
    info!("Starting automatic handshake test");

    let network_name = "test-network";
    let instance_id = "test-instance";
    let allowed_keys = HashSet::new();

    // Create two nodes
    let mut node1 = TestNode::new(network_name, instance_id, allowed_keys.clone(), vec![]).await;
    let mut node2 = TestNode::new(network_name, instance_id, allowed_keys, vec![]).await;

    info!("Starting nodes");
    // Start both nodes - this should trigger automatic handshake
    let handle1 = node1.start().await.expect("Failed to start node1");
    let handle2 = node2.start().await.expect("Failed to start node2");

    // Wait for automatic handshake completion
    info!("Waiting for automatic handshake completion");
    timeout(TEST_TIMEOUT, async {
        loop {
            if handle1.peer_manager.is_peer_verified(&node2.peer_id)
                && handle2.peer_manager.is_peer_verified(&node1.peer_id)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("Automatic handshake verification timed out");

    // Verify peer info and identify info are present
    let peer_info1 = handle1
        .peer_info(&node2.peer_id)
        .expect("Missing peer info for node2");
    let peer_info2 = handle2
        .peer_info(&node1.peer_id)
        .expect("Missing peer info for node1");

    assert!(
        peer_info1.identify_info.is_some(),
        "Missing identify info for node2"
    );
    assert!(
        peer_info2.identify_info.is_some(),
        "Missing identify info for node1"
    );

    info!("Automatic handshake test completed successfully");
}

#[tokio::test]
async fn test_handshake_with_invalid_peer() {
    init_tracing();
    info!("Starting invalid peer handshake test");

    let network_name = "test-network";
    let instance_id = "test-instance";

    // Create node1 with empty whitelist
    let mut node1 = TestNode::new(network_name, instance_id, HashSet::new(), vec![]).await;

    // Create node2 with non-whitelisted key
    let mut node2 = TestNode::new(network_name, instance_id, HashSet::new(), vec![]).await;

    info!("Starting nodes");
    let handle1 = node1.start().await.expect("Failed to start node1");
    let handle2 = node2.start().await.expect("Failed to start node2");

    // Wait for ban to be applied automatically
    info!("Waiting for automatic ban");
    timeout(TEST_TIMEOUT, async {
        loop {
            if handle2.peer_manager.is_banned(&node1.peer_id) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("Ban was not applied");

    // Verify peers remain unverified
    assert!(!handle1.peer_manager.is_peer_verified(&node2.peer_id));
    assert!(!handle2.peer_manager.is_peer_verified(&node1.peer_id));

    info!("Invalid peer handshake test completed successfully");
}

#[tokio::test]
async fn test_handshake_reconnection() {
    init_tracing();
    info!("Starting handshake reconnection test");

    let network_name = "test-network";
    let instance_id = "test-instance";
    let allowed_keys = HashSet::new();

    // Create two nodes
    let mut node1 = TestNode::new(network_name, instance_id, allowed_keys.clone(), vec![]).await;
    let mut node2 = TestNode::new(network_name, instance_id, allowed_keys, vec![]).await;

    info!("Starting initial connection");
    // Start both nodes and wait for initial handshake
    let handle1 = node1.start().await.expect("Failed to start node1");
    let handle2 = node2.start().await.expect("Failed to start node2");

    // Wait for initial handshake
    timeout(TEST_TIMEOUT, async {
        loop {
            if handle1.peer_manager.is_peer_verified(&node2.peer_id)
                && handle2.peer_manager.is_peer_verified(&node1.peer_id)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("Initial handshake timed out");

    info!("Disconnecting node2");
    // Drop node2's handle to simulate disconnect
    drop(handle2);
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify node1 sees node2 as disconnected
    assert!(!handle1.peer_manager.is_peer_verified(&node2.peer_id));

    info!("Reconnecting node2");
    // Restart node2
    let handle2 = node2.start().await.expect("Failed to restart node2");

    // Wait for automatic reconnection and handshake
    info!("Waiting for automatic reconnection handshake");
    timeout(TEST_TIMEOUT, async {
        loop {
            if handle1.peer_manager.is_peer_verified(&node2.peer_id)
                && handle2.peer_manager.is_peer_verified(&node1.peer_id)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("Reconnection handshake timed out");

    info!("Handshake reconnection test completed successfully");
}

// #[tokio::test]
// async fn test_concurrent_connections() {
//     init_tracing();
//     info!("Starting concurrent connections test");

//     let network_name = "test-network";
//     let instance_id = "test-instance";
//     let allowed_keys = HashSet::new();

//     // Create three nodes to test multiple concurrent handshakes
//     let mut node1 = TestNode::new(network_name, instance_id, allowed_keys.clone(), vec![]).await;
//     let mut node2 = TestNode::new(network_name, instance_id, allowed_keys.clone(), vec![]).await;
//     let mut node3 = TestNode::new(network_name, instance_id, allowed_keys, vec![]).await;

//     info!("Starting all nodes simultaneously");
//     // Start all nodes simultaneously
//     let (handle1, handle2, handle3) = tokio::join!(node1.start(), node2.start(), node3.start());
//     let handle1 = handle1.expect("Failed to start node1");
//     let handle2 = handle2.expect("Failed to start node2");
//     let handle3 = handle3.expect("Failed to start node3");

//     // Wait for all handshakes to complete
//     info!("Waiting for all handshakes to complete");
//     timeout(TEST_TIMEOUT, async {
//         loop {
//             let all_verified = handle1.peer_manager.is_peer_verified(&node2.peer_id)
//                 && handle1.peer_manager.is_peer_verified(&node3.peer_id)
//                 && handle2.peer_manager.is_peer_verified(&node1.peer_id)
//                 && handle2.peer_manager.is_peer_verified(&node3.peer_id)
//                 && handle3.peer_manager.is_peer_verified(&node1.peer_id)
//                 && handle3.peer_manager.is_peer_verified(&node2.peer_id);

//             if all_verified {
//                 break;
//             }
//             tokio::time::sleep(Duration::from_millis(100)).await;
//         }
//     })
//     .await
//     .expect("Concurrent handshakes timed out");

//     // Verify all peer info is present
//     for (handle, peers) in [
//         (&handle1, vec![&node2.peer_id, &node3.peer_id]),
//         (&handle2, vec![&node1.peer_id, &node3.peer_id]),
//         (&handle3, vec![&node1.peer_id, &node2.peer_id]),
//     ] {
//         for peer_id in peers {
//             assert!(
//                 handle.peer_info(peer_id).unwrap().identify_info.is_some(),
//                 "Missing identify info for peer {peer_id:?}"
//             );
//         }
//     }

//     info!("Concurrent connections test completed successfully");
// }
