// TODO
// use gadget_crypto::KeyType;
// use libp2p::PeerId;
// use serde::{Deserialize, Serialize};
// use std::{collections::HashSet, time::Duration};
// use tokio::time::timeout;
// use tracing::info;
//
// use crate::{
//     blueprint_protocol::{InstanceMessageRequest, InstanceMessageResponse},
//     key_types::{Curve, InstanceMsgKeyPair, InstanceMsgPublicKey},
//     service::NetworkMessage,
//     service_handle::NetworkServiceHandle,
//     tests::{
//         create_whitelisted_nodes, wait_for_all_handshakes, wait_for_handshake_completion, TestNode,
//     },
//     types::{MessageRouting, ParticipantId, ParticipantInfo, ProtocolMessage},
// };
//
// const TEST_TIMEOUT: Duration = Duration::from_secs(10);
// const PROTOCOL_NAME: &str = "summation/1.0.0";
//
// // Protocol message types
// #[derive(Debug, Clone, Serialize, Deserialize)]
// enum SummationMessage {
//     Number(u64),
//     Verification { sum: u64 },
// }
//
// // Helper to create a protocol message
// fn create_protocol_message(
//     protocol: &str,
//     msg: SummationMessage,
//     sender: &NetworkServiceHandle,
//     recipient: Option<PeerId>,
// ) -> ProtocolMessage {
//     ProtocolMessage {
//         protocol: protocol.to_string(),
//         routing: MessageRouting {
//             message_id: 0,
//             round_id: 0,
//             sender: ParticipantInfo {
//                 id: ParticipantId(0),
//                 public_key: None,
//             },
//             recipient: recipient.map(|peer_id| ParticipantInfo {
//                 id: ParticipantId(0),
//                 public_key: None,
//             }),
//         },
//         payload: bincode::serialize(&msg).expect("Failed to serialize message"),
//     }
// }
//
// // Helper to extract number from message
// fn extract_number_from_message(msg: &ProtocolMessage) -> u64 {
//     match bincode::deserialize::<SummationMessage>(&msg.payload).expect("Failed to deserialize") {
//         SummationMessage::Number(n) => n,
//         _ => panic!("Expected number message"),
//     }
// }
//
// // Helper to extract sum from verification message
// fn extract_sum_from_verification(msg: &ProtocolMessage) -> u64 {
//     match bincode::deserialize::<SummationMessage>(&msg.payload).expect("Failed to deserialize") {
//         SummationMessage::Verification { sum } => sum,
//         _ => panic!("Expected verification message"),
//     }
// }
//
// #[tokio::test]
// async fn test_summation_protocol_basic() {
//     super::init_tracing();
//     info!("Starting summation protocol test");
//
//     // Create nodes with whitelisted keys
//     let instance_key_pair2 = Curve::generate_with_seed(None).unwrap();
//     let mut allowed_keys1 = HashSet::new();
//     allowed_keys1.insert(instance_key_pair2.public());
//
//     let mut node1 = TestNode::new("test-net", "sum-test", allowed_keys1, vec![]).await;
//
//     let mut allowed_keys2 = HashSet::new();
//     allowed_keys2.insert(node1.instance_key_pair.public());
//     let mut node2 = TestNode::new_with_keys(
//         "test-net",
//         "sum-test",
//         allowed_keys2,
//         vec![],
//         Some(instance_key_pair2),
//         None,
//     )
//     .await;
//
//     info!("Starting nodes");
//     let mut handle1 = node1.start().await.expect("Failed to start node1");
//     let mut handle2 = node2.start().await.expect("Failed to start node2");
//
//     info!("Waiting for handshake completion");
//     wait_for_handshake_completion(&handle1, &handle2, TEST_TIMEOUT).await;
//
//     // Generate test numbers
//     let num1 = 42;
//     let num2 = 58;
//     let expected_sum = num1 + num2;
//
//     info!("Sending numbers via gossip");
//     // Send numbers via gossip
//     handle1
//         .send(create_protocol_message(
//             PROTOCOL_NAME,
//             SummationMessage::Number(num1),
//             &handle1,
//             None,
//         ))
//         .expect("Failed to send number from node1");
//
//     handle2
//         .send(create_protocol_message(
//             PROTOCOL_NAME,
//             SummationMessage::Number(num2),
//             &handle2,
//             None,
//         ))
//         .expect("Failed to send number from node2");
//
//     info!("Waiting for messages to be processed");
//     // Wait for messages and compute sums
//     let mut sum1 = 0;
//     let mut sum2 = 0;
//     let mut node1_received = false;
//     let mut node2_received = false;
//
//     timeout(TEST_TIMEOUT, async {
//         loop {
//             // Process incoming messages
//             if let Some(msg) = handle1.next_protocol_message() {
//                 if !node1_received {
//                     sum1 += extract_number_from_message(&msg);
//                     node1_received = true;
//                 }
//             }
//             if let Some(msg) = handle2.next_protocol_message() {
//                 if !node2_received {
//                     sum2 += extract_number_from_message(&msg);
//                     node2_received = true;
//                 }
//             }
//
//             // Check if both nodes have received messages
//             if node1_received && node2_received {
//                 break;
//             }
//             tokio::time::sleep(Duration::from_millis(100)).await;
//         }
//     })
//     .await
//     .expect("Timeout waiting for summation completion");
//
//     info!("Verifying sums via P2P messages");
//     // Verify sums via P2P messages
//     handle1
//         .send(create_protocol_message(
//             PROTOCOL_NAME,
//             SummationMessage::Verification { sum: sum1 },
//             &handle1,
//             Some(node2.peer_id),
//         ))
//         .expect("Failed to send verification from node1");
//
//     handle2
//         .send(create_protocol_message(
//             PROTOCOL_NAME,
//             SummationMessage::Verification { sum: sum2 },
//             &handle2,
//             Some(node1.peer_id),
//         ))
//         .expect("Failed to send verification from node2");
//
//     info!("Waiting for verification messages");
//     // Wait for verification messages
//     timeout(TEST_TIMEOUT, async {
//         let mut node1_verified = false;
//         let mut node2_verified = false;
//
//         loop {
//             // Process verification messages
//             if let Some(msg) = handle1.next_protocol_message() {
//                 if !node1_verified {
//                     assert_eq!(extract_sum_from_verification(&msg), expected_sum);
//                     node1_verified = true;
//                 }
//             }
//             if let Some(msg) = handle2.next_protocol_message() {
//                 if !node2_verified {
//                     assert_eq!(extract_sum_from_verification(&msg), expected_sum);
//                     node2_verified = true;
//                 }
//             }
//
//             if node1_verified && node2_verified {
//                 break;
//             }
//             tokio::time::sleep(Duration::from_millis(100)).await;
//         }
//     })
//     .await
//     .expect("Timeout waiting for verification completion");
//
//     info!("Summation protocol test completed successfully");
// }
//
// #[tokio::test]
// async fn test_summation_protocol_multi_node() {
//     super::init_tracing();
//     info!("Starting multi-node summation protocol test");
//
//     // Create 3 nodes with whitelisted keys
//     info!("Creating whitelisted nodes");
//     let mut nodes = create_whitelisted_nodes(3).await;
//     info!("Created {} nodes successfully", nodes.len());
//
//     // Start all nodes
//     info!("Starting all nodes");
//     let mut handles = Vec::new();
//     for (i, node) in nodes.iter_mut().enumerate() {
//         info!("Starting node {}", i);
//         handles.push(node.start().await.expect("Failed to start node"));
//         info!("Node {} started successfully", i);
//     }
//
//     // Convert handles to mutable references
//     info!("Converting handles to mutable references");
//     let mut handles: Vec<&mut NetworkServiceHandle> = handles.iter_mut().collect();
//     let handles_len = handles.len();
//     info!("Converted {} handles", handles_len);
//
//     // Wait for all handshakes to complete
//     info!(
//         "Waiting for handshake completion between {} nodes",
//         handles_len
//     );
//     wait_for_all_handshakes(&handles, TEST_TIMEOUT).await;
//     info!("All handshakes completed successfully");
//
//     // Generate test numbers
//     let numbers = vec![42, 58, 100];
//     let expected_sum: u64 = numbers.iter().sum();
//     info!(
//         "Generated test numbers: {:?}, expected sum: {}",
//         numbers, expected_sum
//     );
//
//     info!("Sending numbers via gossip");
//     // Each node broadcasts its number
//     for (i, handle) in handles.iter().enumerate() {
//         info!("Node {} broadcasting number {}", i, numbers[i]);
//         handle
//             .send(create_protocol_message(
//                 PROTOCOL_NAME,
//                 SummationMessage::Number(numbers[i]),
//                 handle,
//                 None,
//             ))
//             .expect("Failed to send number");
//         info!("Node {} successfully broadcast its number", i);
//         // Add a small delay between broadcasts to avoid message collisions
//         tokio::time::sleep(Duration::from_millis(100)).await;
//     }
//
//     info!("Waiting for messages to be processed");
//     // Wait for all nodes to receive all numbers
//     let mut sums = vec![0; handles_len];
//     let mut received = vec![0; handles_len];
//
//     timeout(TEST_TIMEOUT, async {
//         loop {
//             for (i, handle) in handles.iter_mut().enumerate() {
//                 if let Some(msg) = handle.next_protocol_message() {
//                     if received[i] < handles_len - 1 {
//                         let num = extract_number_from_message(&msg);
//                         sums[i] += num;
//                         received[i] += 1;
//                         info!(
//                             "Node {} received number {}, total sum: {}, received count: {}",
//                             i, num, sums[i], received[i]
//                         );
//                     }
//                 }
//             }
//
//             let all_received = received.iter().all(|&r| r == handles_len - 1);
//             info!(
//                 "Current received counts: {:?}, target count: {}",
//                 received,
//                 handles_len - 1
//             );
//             if all_received {
//                 info!("All nodes have received all numbers");
//                 break;
//             }
//             tokio::time::sleep(Duration::from_millis(100)).await;
//         }
//     })
//     .await
//     .expect("Timeout waiting for summation completion");
//
//     info!("Verifying sums via P2P messages");
//     info!("Final sums: {:?}", sums);
//     // Each node verifies with every other node
//     for (i, sender) in handles.iter().enumerate() {
//         for (j, recipient) in handles.iter().enumerate() {
//             if i != j {
//                 info!(
//                     "Node {} sending verification sum {} to node {}",
//                     i, sums[i], j
//                 );
//                 sender
//                     .send(create_protocol_message(
//                         PROTOCOL_NAME,
//                         SummationMessage::Verification { sum: sums[i] },
//                         sender,
//                         Some(recipient.local_peer_id),
//                     ))
//                     .expect("Failed to send verification");
//             }
//         }
//     }
//
//     info!("Waiting for verification messages");
//     // Wait for all verifications
//     timeout(TEST_TIMEOUT, async {
//         let mut verified = vec![0; handles_len];
//         loop {
//             for (i, handle) in handles.iter_mut().enumerate() {
//                 if let Some(msg) = handle.next_protocol_message() {
//                     if verified[i] < handles_len - 1 {
//                         let sum = extract_sum_from_verification(&msg);
//                         info!(
//                             "Node {} received verification sum {}, expected {}",
//                             i, sum, expected_sum
//                         );
//                         assert_eq!(sum, expected_sum);
//                         verified[i] += 1;
//                         info!("Node {} verification count: {}", i, verified[i]);
//                     }
//                 }
//             }
//
//             let all_verified = verified.iter().all(|&v| v == handles_len - 1);
//             info!("Current verification counts: {:?}", verified);
//             if all_verified {
//                 info!("All nodes have verified all sums");
//                 break;
//             }
//             tokio::time::sleep(Duration::from_millis(100)).await;
//         }
//     })
//     .await
//     .expect("Timeout waiting for verification completion");
//
//     info!("Multi-node summation protocol test completed successfully");
// }
//
// #[tokio::test]
// async fn test_summation_protocol_late_join() {
//     super::init_tracing();
//     info!("Starting late join summation protocol test");
//
//     // Create 3 nodes but only start 2 initially
//     let mut nodes = create_whitelisted_nodes(3).await;
//
//     // Start first two nodes
//     let mut handles = Vec::new();
//     for node in nodes[..2].iter_mut() {
//         handles.push(node.start().await.expect("Failed to start node"));
//     }
//
//     // Convert handles to mutable references
//     let handles_refs: Vec<&mut NetworkServiceHandle> = handles.iter_mut().collect();
//
//     // Wait for initial handshakes
//     info!("Waiting for initial handshake completion");
//     wait_for_all_handshakes(&handles_refs, TEST_TIMEOUT).await;
//
//     // Initial nodes send their numbers
//     let numbers = vec![42, 58, 100];
//     let _expected_sum: u64 = numbers.iter().sum();
//
//     info!("Initial nodes sending numbers");
//     for (i, handle) in handles.iter().enumerate() {
//         handle
//             .send(create_protocol_message(
//                 PROTOCOL_NAME,
//                 SummationMessage::Number(numbers[i]),
//                 handle,
//                 None,
//             ))
//             .expect("Failed to send number");
//     }
//
//     // Wait for initial nodes to process messages
//     timeout(TEST_TIMEOUT, async {
//         let mut received = vec![false; 2];
//         loop {
//             for (i, handle) in handles.iter_mut().enumerate() {
//                 if let Some(msg) = handle.next_protocol_message() {
//                     if !received[i] {
//                         assert_eq!(extract_number_from_message(&msg), numbers[1 - i]);
//                         received[i] = true;
//                     }
//                 }
//             }
//             if received.iter().all(|&r| r) {
//                 break;
//             }
//             tokio::time::sleep(Duration::from_millis(100)).await;
//         }
//     })
//     .await
//     .expect("Timeout waiting for initial summation");
//
//     // Start the late joining node
//     info!("Starting late joining node");
//     handles.push(nodes[2].start().await.expect("Failed to start late node"));
//     let all_handles: Vec<&mut NetworkServiceHandle> = handles.iter_mut().collect();
//
//     // Wait for the new node to complete handshakes
//     wait_for_all_handshakes(&all_handles, TEST_TIMEOUT).await;
//
//     // Late node sends its number and receives history
//     info!("Late node sending number and receiving history");
//     handles[2]
//         .send(create_protocol_message(
//             PROTOCOL_NAME,
//             SummationMessage::Number(numbers[2]),
//             &handles[2],
//             None,
//         ))
//         .expect("Failed to send number from late node");
//
//     // Verify final state
//     timeout(TEST_TIMEOUT, async {
//         let mut verified = vec![false; handles.len()];
//         loop {
//             for (i, handle) in handles.iter_mut().enumerate() {
//                 if let Some(msg) = handle.next_protocol_message() {
//                     let num = extract_number_from_message(&msg);
//                     if !verified[i] && (num == numbers[2] || numbers.contains(&num)) {
//                         verified[i] = true;
//                     }
//                 }
//             }
//             if verified.iter().all(|&v| v) {
//                 break;
//             }
//             tokio::time::sleep(Duration::from_millis(100)).await;
//         }
//     })
//     .await
//     .expect("Timeout waiting for late node synchronization");
//
//     info!("Late join test completed successfully");
// }
//
// #[tokio::test]
// async fn test_summation_protocol_node_disconnect() {
//     super::init_tracing();
//     info!("Starting node disconnect test");
//
//     // Create 3 nodes
//     let mut nodes = create_whitelisted_nodes(3).await;
//
//     // Start all nodes
//     let mut handles = Vec::new();
//     for node in nodes.iter_mut() {
//         handles.push(node.start().await.expect("Failed to start node"));
//     }
//
//     // Convert handles to mutable references
//     let handles_refs: Vec<&mut NetworkServiceHandle> = handles.iter_mut().collect();
//
//     // Wait for all handshakes
//     wait_for_all_handshakes(&handles_refs, TEST_TIMEOUT).await;
//
//     // Send initial numbers
//     let numbers = vec![42, 58, 100];
//     for (i, handle) in handles.iter().enumerate() {
//         handle
//             .send(create_protocol_message(
//                 PROTOCOL_NAME,
//                 SummationMessage::Number(numbers[i]),
//                 handle,
//                 None,
//             ))
//             .expect("Failed to send number");
//     }
//
//     // Wait for initial processing
//     timeout(TEST_TIMEOUT, async {
//         let mut received = vec![0; handles.len()];
//         loop {
//             for (i, handle) in handles.iter_mut().enumerate() {
//                 if let Some(msg) = handle.next_protocol_message() {
//                     if received[i] < 2 {
//                         extract_number_from_message(&msg);
//                         received[i] += 1;
//                     }
//                 }
//             }
//             if received.iter().all(|&r| r == 2) {
//                 break;
//             }
//             tokio::time::sleep(Duration::from_millis(100)).await;
//         }
//     })
//     .await
//     .expect("Timeout waiting for initial messages");
//
//     // Disconnect one node
//     info!("Disconnecting node");
//     drop(handles.pop());
//     let mut remaining_handles: Vec<&mut NetworkServiceHandle> = handles.iter_mut().collect();
//
//     // Verify remaining nodes can still communicate
//     info!("Verifying remaining nodes can communicate");
//     for (i, sender) in remaining_handles.iter().enumerate() {
//         for (j, recipient) in remaining_handles.iter().enumerate() {
//             if i != j {
//                 sender
//                     .send(create_protocol_message(
//                         PROTOCOL_NAME,
//                         SummationMessage::Verification { sum: numbers[i] },
//                         sender,
//                         Some(recipient.local_peer_id),
//                     ))
//                     .expect("Failed to send verification");
//             }
//         }
//     }
//
//     // Wait for verification messages
//     timeout(TEST_TIMEOUT, async {
//         let mut verified = vec![false; remaining_handles.len()];
//         loop {
//             for (i, handle) in remaining_handles.iter_mut().enumerate() {
//                 if let Some(msg) = handle.next_protocol_message() {
//                     if !verified[i] {
//                         extract_sum_from_verification(&msg);
//                         verified[i] = true;
//                     }
//                 }
//             }
//             if verified.iter().all(|&v| v) {
//                 break;
//             }
//             tokio::time::sleep(Duration::from_millis(100)).await;
//         }
//     })
//     .await
//     .expect("Timeout waiting for verification after disconnect");
//
//     info!("Node disconnect test completed successfully");
// }
