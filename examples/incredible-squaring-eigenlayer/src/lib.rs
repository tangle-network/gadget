//! Incredible Squaring TaskManager Monitor
//!
//! Monitors TaskManager events for task creation and completion.

use ::std::sync::Arc;

use crate::config::Keystore;
use crate::contracts::IBLSSignatureChecker::NonSignerStakesAndSignature;
use crate::contracts::SquaringTask::{self, NewTaskCreated};
use crate::contracts::TaskManager::TaskResponse;
use crate::contracts::BN254::{G1Point, G2Point};
use crate::error::TaskError;
use alloy_primitives::{keccak256, U256};
use alloy_provider::RootProvider;
use alloy_sol_types::{SolEvent, SolType, SolValue};
use blueprint_evm_extra::extract::{BlockNumber, ContractAddress, FirstEvent, Tx};
use blueprint_evm_extra::filters::{contract::MatchesContract, event::MatchesEvent};
use blueprint_sdk::extract::Context;
use blueprint_sdk::*;
use eigensdk::crypto_bls::{
    convert_to_g1_point, convert_to_g2_point, BlsG1Point, BlsG2Point, BlsKeyPair,
};
use eigensdk::services_blsaggregation::bls_agg;
use eigensdk::services_blsaggregation::bls_agg::{TaskMetadata, TaskSignature};
use eigensdk::types::operator::OperatorId;
use tokio::sync::Mutex;
use tower::filter::FilterLayer;

pub mod config;
pub mod contracts;
pub mod error;

/// Service context shared between jobs
#[derive(Debug, Clone)]
pub struct ExampleContext {
    provider: Arc<RootProvider>,
    keystore: Keystore,
    bls_aggregation_service: Arc<Mutex<(bls_agg::ServiceHandle, bls_agg::AggregateReceiver)>>,
}

/// Job function for handling tasks
#[blueprint_sdk::debug_job]
pub async fn handle_task(
    Context(ctx): Context<ExampleContext>,
    BlockNumber(block_number): BlockNumber,
    ContractAddress(addr): ContractAddress,
    FirstEvent(ev): FirstEvent<NewTaskCreated>,
) -> Result<Tx, TaskError> {
    tracing::info!(
        "Processing task: index={}, quorum={}, block_number={}, contract={}",
        ev.taskIndex,
        ev.task.quorumNumbers,
        block_number,
        addr
    );

    let contract = SquaringTask::new(addr, &ctx.provider);
    let number = U256::abi_decode(&ev.task.message, true)?;
    let result = number * number;

    tracing::info!(%number, %result, "Calculated result");
    let task_response = TaskResponse {
        referenceTaskIndex: ev.taskIndex,
        message: result.abi_encode().into(),
    };

    // Initialize the task.
    let mut lock = ctx.bls_aggregation_service.lock().await;
    let time_to_expiry = std::time::Duration::from_secs(1200);
    lock.0
        .initialize_task(TaskMetadata::new(
            ev.taskIndex,
            ev.task.taskCreatedBlock as u64,
            ev.task.quorumNumbers.to_vec(),
            vec![
                ev.task.quorumThresholdPercentage.try_into().unwrap();
                ev.task.quorumNumbers.len()
            ],
            time_to_expiry,
        ))
        .await?;

    let msg_hash = keccak256(<TaskResponse as SolType>::abi_encode(&task_response));
    let bls_key_pair = ctx.keystore.bls_keypair();
    let operator_id = operator_id_from_key(bls_key_pair.clone());
    let sig = bls_key_pair.sign_message(&msg_hash);
    lock.0
        .process_signature(TaskSignature::new(ev.taskIndex, msg_hash, sig, operator_id))
        .await?;
    let response = lock
        .1
        .receive_aggregated_response()
        .await
        .map_err(|_| TaskError::AggregatedResponseReceiverClosed)?;
    let non_signer_stakes_and_signature = NonSignerStakesAndSignature {
        nonSignerPubkeys: response
            .non_signers_pub_keys_g1
            .into_iter()
            .map(to_g1_point)
            .collect(),
        nonSignerQuorumBitmapIndices: response.non_signer_quorum_bitmap_indices,
        quorumApks: response
            .quorum_apks_g1
            .into_iter()
            .map(to_g1_point)
            .collect(),
        apkG2: to_g2_point(response.signers_apk_g2),
        sigma: to_g1_point(response.signers_agg_sig_g1.g1_point()),
        quorumApkIndices: response.quorum_apk_indices,
        totalStakeIndices: response.total_stake_indices,
        nonSignerStakeIndices: response.non_signer_stake_indices,
    };
    let tx = contract
        .respondToSquaringTask(ev.task, task_response, non_signer_stakes_and_signature)
        .into_transaction_request();

    Ok(Tx(tx))
}

/// Creates a router with task event filters
pub fn create_contract_router(
    ctx: ExampleContext,
    contract_address: alloy_primitives::Address,
) -> Router {
    let sig = NewTaskCreated::SIGNATURE_HASH;
    Router::new()
        .route(*sig, handle_task)
        .with_context(ctx)
        .layer(FilterLayer::new(MatchesEvent(sig)))
        .layer(FilterLayer::new(MatchesContract(contract_address)))
}

/// Generate the Operator ID from the BLS Keypair
pub fn operator_id_from_key(key: BlsKeyPair) -> OperatorId {
    let pub_key = key.public_key();
    let pub_key_affine = pub_key.g1();

    let x_int: num_bigint::BigUint = pub_key_affine.x.into();
    let y_int: num_bigint::BigUint = pub_key_affine.y.into();

    let x_bytes = x_int.to_bytes_be();
    let y_bytes = y_int.to_bytes_be();

    keccak256([x_bytes, y_bytes].concat())
}

fn to_g1_point(pk: BlsG1Point) -> G1Point {
    let pt = convert_to_g1_point(pk.g1()).expect("Invalid G1 point");
    G1Point { X: pt.X, Y: pt.Y }
}

fn to_g2_point(pk: BlsG2Point) -> G2Point {
    let pt = convert_to_g2_point(pk.g2()).expect("Invalid G2 point");
    G2Point { X: pt.X, Y: pt.Y }
}

#[cfg(test)]
mod tests {
    use crate::config::EigenlayerBLSConfig;
    use crate::{config::get_provider_http, contracts::SquaringTask};

    use super::*;
    use alloy_network::EthereumWallet;
    use alloy_primitives::{address, Address, U256};
    use alloy_provider::Provider;
    use alloy_signer_local::PrivateKeySigner;
    use blueprint_evm_extra::consumer::EVMConsumer;
    use blueprint_evm_extra::producer::{PollingConfig, PollingProducer};
    use blueprint_runner::config::{BlueprintEnvironment, ProtocolSettings};
    use blueprint_runner::BlueprintRunner;
    use blueprint_sdk::error::BoxError;
    use gadget_anvil_testing_utils::keys::ANVIL_PRIVATE_KEYS;
    use gadget_anvil_testing_utils::start_default_anvil_testnet;
    use gadget_logging::setup_log;
    use std::{sync::Arc, time::Duration};

    const REGISTRY_COORDINATOR_ADDRESS: Address =
        address!("0xc3e53f4d16ae77db1c982e75a937b9f60fe63690");

    /// Address of the first account in the local anvil network
    const ANVIL_FIRST_ADDRESS: Address = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");

    /// Submits squaring tasks periodically
    async fn submit_tasks<P: Provider>(
        provider: P,
        contract_address: alloy_primitives::Address,
    ) -> Result<(), BoxError> {
        let mut interval = tokio::time::interval(Duration::from_secs(3));
        let mut number: u64 = 1;

        loop {
            interval.tick().await;

            // Create task parameters
            let quorum_threshold = 66; // 66% threshold
            let quorum_numbers = vec![0u8]; // Example quorum number

            let contract = SquaringTask::new(contract_address, &provider);
            // Submit task
            match contract
                .createSquaringTask(U256::from(number), quorum_threshold, quorum_numbers.into())
                .send()
                .await
            {
                Ok(tx) => {
                    let receipt = tx.get_receipt().await?;
                    assert!(receipt.status(), "Failed to process receipt: {:?}", receipt);
                    tracing::info!(
                        "Submitted squaring task for number {} in tx {:?} (block: {:?})",
                        number,
                        receipt.transaction_hash,
                        receipt.block_number,
                    );
                    number += 1;
                }
                Err(e) => {
                    tracing::error!("Failed to submit task: {}", e);
                }
            }
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn task_monitoring() -> Result<(), BoxError> {
        setup_log();
        let (anvil_container, rpc_url, ws_url) = start_default_anvil_testnet(false).await;
        let mut env = BlueprintEnvironment::default();
        env.http_rpc_endpoint = rpc_url.clone();
        env.ws_rpc_endpoint = ws_url.clone();
        let el_settings = Default::default();
        env.protocol_settings = ProtocolSettings::Eigenlayer(el_settings);
        let signer: PrivateKeySigner = ANVIL_PRIVATE_KEYS[0].parse().unwrap();

        let provider = get_provider_http(&rpc_url);

        // Deploy contracts and get addresses
        let generator = ANVIL_FIRST_ADDRESS;
        let aggregator = ANVIL_FIRST_ADDRESS;
        let owner = ANVIL_FIRST_ADDRESS;

        let register_coordinatior = REGISTRY_COORDINATOR_ADDRESS;
        let task_response_window = 1000;
        let avs_registry_chain_reader =
            eigensdk::client_avsregistry::reader::AvsRegistryChainReader::new(
                eigensdk::logging::get_test_logger(),
                register_coordinatior,
                el_settings.operator_state_retriever_address,
                rpc_url,
            )
            .await?;
        let (operator_info_service, _) = eigensdk::services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceInMemory::new(
            eigensdk::logging::get_test_logger(),
            avs_registry_chain_reader.clone(),
            ws_url,
        )
        .await?;
        let cancellation_token = tokio_util::sync::CancellationToken::new();
        let token_clone = cancellation_token.clone();
        let provider = provider.clone();
        let current_block = provider.get_block_number().await?;
        let operator_info_clone = operator_info_service.clone();

        tokio::task::spawn(async move {
            operator_info_clone
                .start_service(&token_clone, 0, current_block)
                .await
        });
        let avs_registry_service_chain_caller =
            eigensdk::services_avsregistry::chaincaller::AvsRegistryServiceChainCaller::new(
                avs_registry_chain_reader,
                operator_info_service,
            );
        let bls_aggregator_service =
            eigensdk::services_blsaggregation::bls_agg::BlsAggregatorService::new(
                avs_registry_service_chain_caller,
                eigensdk::logging::get_test_logger(),
            );
        println!("Deploying contract");
        let contract = SquaringTask::deploy(
            provider.clone(),
            register_coordinatior,
            task_response_window,
        )
        .await?;
        println!("Contract deployed at: {:?}", contract.address());
        let receipt = contract
            .initialize(aggregator, generator, owner)
            .send()
            .await?
            .get_receipt()
            .await?;

        assert!(receipt.status(), "{receipt:?}");

        let provider = Arc::new(provider);
        // Create producer for task events
        let task_polling_producer = PollingProducer::new(
            provider.clone(),
            PollingConfig {
                poll_interval: Duration::from_millis(2000),
                start_block: 200,
                confirmations: 3,
                step: 1,
            },
        );

        let evm_consumer = EVMConsumer::new(provider.clone(), EthereumWallet::new(signer));

        // Submit task
        let handle = tokio::spawn(submit_tasks(provider.clone(), *contract.address()));
        let config = EigenlayerBLSConfig::new(ANVIL_FIRST_ADDRESS, ANVIL_FIRST_ADDRESS)
            .with_exit_after_register(false);

        // Create and run the blueprint
        BlueprintRunner::builder(config, env)
            .router(create_contract_router(
                ExampleContext {
                    provider: provider.clone(),
                    keystore: Keystore::default(),
                    bls_aggregation_service: Arc::new(Mutex::new(bls_aggregator_service.start())),
                },
                *contract.address(),
            ))
            .producer(task_polling_producer)
            .consumer(evm_consumer)
            .with_shutdown_handler(async move {
                handle.abort();
                anvil_container.stop().await.unwrap();
                anvil_container.rm().await.unwrap();
            })
            .run()
            .await?;

        Ok(())
    }
}
