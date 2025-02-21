use crate::{key_types::Curve, tests::TestNode};
use gadget_crypto::KeyType;
use std::{collections::HashSet, time::Duration};
use tokio::time::timeout;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

const TEST_TIMEOUT: Duration = Duration::from_secs(5);

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
async fn test_automatic_handshake() {
    init_tracing();
    info!("Starting automatic handshake test");

    let network_name = "test-network";
    let instance_id = "test-instance";

    // Generate node2's key pair first
    let instance_key_pair2 = Curve::generate_with_seed(None).unwrap();
    let mut allowed_keys1 = HashSet::new();
    allowed_keys1.insert(instance_key_pair2.public());

    // Create node1 with node2's key whitelisted
    let mut node1 = TestNode::new(network_name, instance_id, allowed_keys1, vec![]);

    // Create node2 with node1's key whitelisted and pre-generated key
    let mut allowed_keys2 = HashSet::new();
    allowed_keys2.insert(node1.instance_key_pair.public());
    let mut node2 = TestNode::new_with_keys(
        network_name,
        instance_id,
        allowed_keys2,
        vec![],
        Some(instance_key_pair2),
        None,
    );

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
    let mut node1 = TestNode::new(network_name, instance_id, HashSet::new(), vec![]);

    // Create node2 with node1's key whitelisted (but node2's key is not whitelisted by node1)
    let mut allowed_keys2 = HashSet::new();
    allowed_keys2.insert(node1.instance_key_pair.public());
    let mut node2 = TestNode::new(network_name, instance_id, allowed_keys2, vec![]);

    info!("Starting nodes");
    let handle1 = node1.start().await.expect("Failed to start node1");
    let handle2 = node2.start().await.expect("Failed to start node2");

    // Wait for ban to be applied automatically
    info!("Waiting for automatic ban");
    timeout(TEST_TIMEOUT, async {
        loop {
            if handle1.peer_manager.is_banned(&node2.peer_id) {
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
