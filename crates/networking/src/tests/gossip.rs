use super::{init_tracing, wait_for_handshake_completion, TestNode};
use crate::{
    service_handle::NetworkServiceHandle,
    tests::{create_whitelisted_nodes, wait_for_all_handshakes},
    types::{MessageRouting, ParticipantId, ParticipantInfo},
};
use std::{collections::HashSet, time::Duration};
use tokio::time::timeout;
use tracing::info;

const TEST_TIMEOUT: Duration = Duration::from_secs(10);
const PROTOCOL_NAME: &str = "/test-net/sum-test";

#[tokio::test]
async fn test_gossip_between_verified_peers() {
    init_tracing();
    info!("Starting gossip test between verified peers");

    // Create nodes with whitelisted keys
    let mut nodes = create_whitelisted_nodes(2).await;
    let mut node2 = nodes.pop().unwrap();
    let mut node1 = nodes.pop().unwrap();

    info!("Starting nodes");
    let handle1 = node1.start().await.expect("Failed to start node1");
    let mut handle2 = node2.start().await.expect("Failed to start node2");

    info!("Waiting for handshake completion");
    wait_for_handshake_completion(&handle1, &handle2, TEST_TIMEOUT).await;

    // Create test message
    info!("Sending gossip message from node1");

    let test_payload = b"Hello, gossip network!".to_vec();
    let routing = MessageRouting {
        message_id: 1,
        round_id: 0,
        sender: ParticipantInfo {
            id: ParticipantId(1),
            public_key: Some(node1.instance_key_pair.public()),
        },
        recipient: None, // No specific recipient for gossip
    };

    handle1
        .send(routing, test_payload.clone())
        .expect("Failed to send gossip message");

    info!("Waiting for node2 to receive the message");
    // Wait for node2 to receive the message
    let received_message = timeout(TEST_TIMEOUT, async {
        loop {
            if let Some(msg) = handle2.next_protocol_message() {
                if msg.protocol == PROTOCOL_NAME {
                    return msg;
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("Timeout waiting for gossip message");

    // Verify message contents
    assert_eq!(received_message.payload, test_payload);
    assert_eq!(received_message.protocol, PROTOCOL_NAME);
    assert_eq!(received_message.routing.message_id, 1);
    assert_eq!(received_message.routing.round_id, 0);
    assert_eq!(
        received_message.routing.sender.public_key,
        Some(node1.instance_key_pair.public())
    );
    assert!(received_message.routing.recipient.is_none());

    info!("Gossip test completed successfully");
}

#[tokio::test]
async fn test_multi_node_gossip() {
    init_tracing();
    info!("Starting multi-node gossip test");

    // Create three nodes with all keys whitelisted
    let mut nodes = create_whitelisted_nodes(3).await;

    info!("Starting all nodes");
    let mut handles: Vec<_> = Vec::new();
    for node in &mut nodes {
        handles.push(node.start().await.expect("Failed to start node"));
    }

    info!("Waiting for all handshakes to complete");
    let handles_refs: Vec<&mut NetworkServiceHandle> = handles.iter_mut().collect();
    wait_for_all_handshakes(&handles_refs, TEST_TIMEOUT).await;

    // Create test message
    let test_payload = b"Multi-node gossip test".to_vec();
    let routing = MessageRouting {
        message_id: 1,
        round_id: 0,
        sender: ParticipantInfo {
            id: ParticipantId(0),
            public_key: Some(nodes[0].instance_key_pair.public()),
        },
        recipient: None,
    };

    info!("Sending gossip message from node 0");
    handles[0]
        .send(routing, test_payload.clone())
        .expect("Failed to send gossip message");

    info!("Waiting for all nodes to receive the message");
    // Wait for all other nodes to receive the message
    timeout(TEST_TIMEOUT, async {
        for (i, handle) in handles.iter_mut().enumerate().skip(1) {
            let received = loop {
                if let Some(msg) = handle.next_protocol_message() {
                    if msg.protocol == PROTOCOL_NAME {
                        break msg;
                    }
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            };

            assert_eq!(
                received.payload, test_payload,
                "Node {} received wrong payload",
                i
            );
            assert_eq!(received.protocol, PROTOCOL_NAME);
            assert_eq!(
                received.routing.sender.public_key,
                Some(nodes[0].instance_key_pair.public())
            );
            info!("Node {} received the gossip message correctly", i);
        }
    })
    .await
    .expect("Timeout waiting for gossip messages");

    info!("Multi-node gossip test completed successfully");
}

#[tokio::test]
async fn test_unverified_peer_gossip() {
    init_tracing();
    info!("Starting unverified peer gossip test");

    // Create two nodes with no whitelisted keys
    let mut node1 = TestNode::new("test-net", "gossip-test", HashSet::new(), vec![]);
    let mut node2 = TestNode::new("test-net", "gossip-test", HashSet::new(), vec![]);

    info!("Starting nodes");
    let handle1 = node1.start().await.expect("Failed to start node1");
    let mut handle2 = node2.start().await.expect("Failed to start node2");

    // Create test message
    let test_payload = b"This message should not be received".to_vec();
    let routing = MessageRouting {
        message_id: 1,
        round_id: 0,
        sender: ParticipantInfo {
            id: ParticipantId(1),
            public_key: Some(node1.instance_key_pair.public()),
        },
        recipient: None,
    };

    info!("Attempting to send gossip message from unverified node");
    handle1
        .send(routing, test_payload.clone())
        .expect("Failed to send gossip message");

    // Wait a bit to ensure message is not received
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Verify node2 did not receive the message
    assert!(handle2.next_protocol_message().is_none());

    info!("Unverified peer gossip test completed successfully");
}
