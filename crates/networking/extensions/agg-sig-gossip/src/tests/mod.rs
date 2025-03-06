#![allow(clippy::too_many_lines)]

use crate::{
    protocol::{AggregationConfig, SignatureAggregationProtocol},
    signature_weight::EqualWeight,
};
use gadget_crypto::aggregation::AggregatableSignature;
use gadget_logging::info;
use gadget_networking::{
    service_handle::NetworkServiceHandle,
    test_utils::{create_whitelisted_nodes, init_tracing, wait_for_all_handshakes},
    types::ParticipantId,
};
use std::{collections::HashMap, time::Duration};

// Constants for tests
const TEST_TIMEOUT: Duration = Duration::from_secs(10);
const PROTOCOL_NAME: &str = "signature-aggregation/1.0.0";

// Generic function to run a signature aggregation test with any signature type
async fn run_signature_aggregation_test<S: AggregatableSignature + 'static>(
    num_nodes: usize,
    threshold_percentage: u8,
    generate_keys_fn: impl Fn(usize) -> Vec<S::Secret>,
    malicious_nodes: Vec<usize>,
) where
    S::Secret: Clone,
    S::Public: Clone,
    S::Signature: Clone,
{
    init_tracing();
    info!(
        "Starting signature aggregation test with {} nodes",
        num_nodes
    );

    // Create whitelisted nodes
    let mut nodes = create_whitelisted_nodes::<S>(num_nodes, false).await;
    info!("Created {} nodes successfully", nodes.len());

    // Start all nodes
    info!("Starting all nodes");
    let mut handles = Vec::new();
    for (i, node) in nodes.iter_mut().enumerate() {
        info!("Starting node {}", i);
        handles.push(node.start().await.expect("Failed to start node"));
        info!("Node {} started successfully", i);
    }

    // Convert handles to mutable references for wait_for_all_handshakes
    let handle_refs: Vec<&mut NetworkServiceHandle<S>> = handles.iter_mut().collect();

    // Wait for all handshakes to complete
    info!(
        "Waiting for handshake completion between {} nodes",
        nodes.len()
    );
    wait_for_all_handshakes(&handle_refs, TEST_TIMEOUT).await;
    info!("All handshakes completed successfully");

    // Generate keys for the signature aggregation protocol
    let secrets = generate_keys_fn(num_nodes);
    let mut public_keys = HashMap::new();
    for i in 0..num_nodes {
        let public_key = S::public_from_secret(&secrets[i]);
        public_keys.insert(ParticipantId(i as u16), public_key);
    }

    // Test messages
    let regular_message = b"test message".to_vec();
    let malicious_message = b"different message".to_vec();

    // Run the protocol directly on each node
    let mut results = Vec::new();
    for i in 0..num_nodes {
        let message = if malicious_nodes.contains(&i) {
            malicious_message.clone()
        } else {
            regular_message.clone()
        };

        let config = AggregationConfig {
            local_id: ParticipantId(i as u16),
            max_participants: num_nodes as u16,
            num_aggregators: 1, // Use single aggregator for simplicity
            timeout: Duration::from_secs(5),
            protocol_id: PROTOCOL_NAME.to_string(),
        };

        let weight_scheme = EqualWeight::new(num_nodes, threshold_percentage);

        let mut protocol = SignatureAggregationProtocol::new(config, weight_scheme);

        let mut secret = secrets[i].clone();
        let handle = handles[i].clone();

        let public_keys_clone = public_keys.clone();
        let result = tokio::spawn(async move {
            protocol
                .run(message, &mut secret, &public_keys_clone, &handle)
                .await
        });

        results.push(result);
    }

    // Wait for results
    let final_results = futures::future::join_all(results).await;

    // Process results
    for (i, result) in final_results.iter().enumerate() {
        if malicious_nodes.contains(&i) {
            // Malicious nodes may fail or have different results
            match result {
                Ok(Ok((res, _))) => {
                    info!(
                        "Malicious node {} completed with {} contributors",
                        i,
                        res.contributors.len()
                    );
                    // Usually only itself in the contributors
                    assert!(
                        res.contributors.contains(&ParticipantId(i as u16)),
                        "Malicious node should include itself as contributor"
                    );
                }
                Ok(Err(e)) => {
                    info!("Malicious node {} failed as expected: {:?}", i, e);
                }
                Err(e) => {
                    panic!("Task for malicious node {} panicked: {:?}", i, e);
                }
            }
        } else {
            // Honest nodes should succeed and agree on the result
            match result {
                Ok(Ok((res, _))) => {
                    // Should exclude malicious nodes
                    for malicious_idx in &malicious_nodes {
                        assert!(
                            res.malicious_participants
                                .contains(&ParticipantId(*malicious_idx as u16)),
                            "Node {} should detect node {} as malicious",
                            i,
                            malicious_idx
                        );
                        assert!(
                            !res.contributors
                                .contains(&ParticipantId(*malicious_idx as u16)),
                            "Node {} should exclude malicious node {} from contributors",
                            i,
                            malicious_idx
                        );
                    }

                    // Should include all honest nodes
                    let expected_contributors = num_nodes - malicious_nodes.len();
                    assert_eq!(
                        res.contributors.len(),
                        expected_contributors,
                        "Node {} should have {} contributors",
                        i,
                        expected_contributors
                    );

                    // Verify signature
                    let mut pub_keys_vec = Vec::new();
                    for id in &res.contributors {
                        pub_keys_vec.push(public_keys.get(id).unwrap().clone());
                    }

                    assert!(
                        S::verify_aggregate(&regular_message, &res.signature, &pub_keys_vec),
                        "Aggregate signature for node {} should be valid",
                        i
                    );
                }
                Ok(Err(e)) => {
                    panic!("Honest node {} failed: {:?}", i, e);
                }
                Err(e) => {
                    panic!("Task for honest node {} panicked: {:?}", i, e);
                }
            }
        }
    }

    info!("Signature aggregation test completed successfully");
}

// BLS Tests
mod bls_tests {
    use super::*;
    use gadget_crypto::{
        sp_core::{SpBls381, SpBls381Pair},
        KeyType,
    };

    fn generate_bls_test_keys(num_keys: usize) -> Vec<SpBls381Pair> {
        let mut keys = Vec::with_capacity(num_keys);
        for i in 0..num_keys {
            let seed = [i as u8; 32];
            keys.push(SpBls381::generate_with_seed(Some(&seed)).unwrap());
        }
        keys
    }

    #[tokio::test]
    async fn test_bls381_basic_aggregation() {
        run_signature_aggregation_test::<SpBls381>(
            3,  // 3 nodes
            67, // 67% threshold (2 out of 3)
            generate_bls_test_keys,
            vec![], // No malicious nodes
        )
        .await;
    }

    #[tokio::test]
    async fn test_bls381_malicious_participant() {
        run_signature_aggregation_test::<SpBls381>(
            4,  // 4 nodes
            75, // 75% threshold (3 out of 4)
            generate_bls_test_keys,
            vec![3], // Node 3 is malicious
        )
        .await;
    }
}

// BN254 Tests
mod bn254_tests {
    use super::*;
    use gadget_crypto::bn254::{ArkBlsBn254, ArkBlsBn254Secret};
    use gadget_crypto::KeyType;

    fn generate_bn254_test_keys(num_keys: usize) -> Vec<ArkBlsBn254Secret> {
        let mut keys = Vec::with_capacity(num_keys);
        for i in 0..num_keys {
            let seed = [i as u8; 32];
            keys.push(ArkBlsBn254::generate_with_seed(Some(&seed)).unwrap());
        }
        keys
    }

    #[tokio::test]
    async fn test_bn254_basic_aggregation() {
        run_signature_aggregation_test::<ArkBlsBn254>(
            3,  // 3 nodes
            67, // 67% threshold (2 out of 3)
            generate_bn254_test_keys,
            vec![], // No malicious nodes
        )
        .await;
    }

    #[tokio::test]
    async fn test_bn254_malicious_participant() {
        run_signature_aggregation_test::<ArkBlsBn254>(
            4,  // 4 nodes
            75, // 75% threshold (3 out of 4)
            generate_bn254_test_keys,
            vec![3], // Node 3 is malicious
        )
        .await;
    }
}
