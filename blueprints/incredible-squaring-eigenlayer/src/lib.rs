#![allow(dead_code)]
use alloy_contract::ContractInstance;
use alloy_network::Ethereum;
use alloy_network::EthereumWallet;
use alloy_network::TransactionBuilder;
use alloy_primitives::keccak256;
use alloy_primitives::{address, hex, Address, Bytes, FixedBytes, Keccak256, U256};
use alloy_provider::fillers::WalletFiller;
use alloy_provider::fillers::{ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller};
use alloy_provider::RootProvider;
use alloy_provider::{Identity, Provider};
use alloy_rpc_types_eth::TransactionRequest;
use alloy_sol_types::SolCall;
use alloy_sol_types::SolType;
use alloy_sol_types::{private::alloy_json_abi::JsonAbi, sol};
use alloy_transport_http::{Client, Http};
use ark_bn254::{Fq, G2Affine};
use ark_ec::AffineRepr;
use ark_ff::{BigInt, BigInteger, PrimeField};
use color_eyre::{eyre::eyre, Result};
use eigensdk::chainio_txmanager::simple_tx_manager::SimpleTxManager;
use eigensdk::client_avsregistry::reader::AvsRegistryChainReader;
use eigensdk::client_avsregistry::writer::AvsRegistryChainWriter;
use eigensdk::crypto_bls::BlsKeyPair;
use eigensdk::crypto_bn254::utils::map_to_curve;
use eigensdk::logging::get_test_logger;
use eigensdk::services_avsregistry::chaincaller;
use eigensdk::services_blsaggregation::bls_agg;
use eigensdk::services_operatorsinfo::operatorsinfo_inmemory;
use gadget_sdk::{events_watcher::evm::Config, info, job};
use std::str::FromStr;
use std::{convert::Infallible, env, ops::Deref, sync::OnceLock};
use structopt::lazy_static::lazy_static;

use k256::sha2::{self, Digest};
use IncredibleSquaringTaskManager::{
    respondToTaskCall, NonSignerStakesAndSignature, Task, TaskResponse,
};

pub mod constants;

// Codegen from ABI file to interact with the contract.
sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    IncredibleSquaringTaskManager,
    "contracts/out/IncredibleSquaringTaskManager.sol/IncredibleSquaringTaskManager.json"
);

#[derive(Debug, Clone)]
pub struct NodeConfig {}

impl Config for NodeConfig {
    type TH = Http<Client>;
    type PH = FillProvider<
        JoinFill<
            JoinFill<JoinFill<JoinFill<Identity, GasFiller>, NonceFiller>, ChainIdFiller>,
            WalletFiller<EthereumWallet>,
        >,
        RootProvider<Http<Client>>,
        Http<Client>,
        Ethereum,
    >;
}

/// Returns x^2 saturating to [`u64::MAX`] if overflow occurs.
#[job(
    id = 1,
    params(number_to_be_squared, task_created_block, quorum_numbers, quorum_threshold_percentage),
    result(_),
    event_listener(EvmContractEventListener(
        instance = IncredibleSquaringTaskManager,
        event = IncredibleSquaringTaskManager::NewTaskCreated,
        event_converter = convert_event_to_inputs,
        callback = IncredibleSquaringTaskManager::IncredibleSquaringTaskManagerCalls::respondToTask
    )),
)]
pub async fn xsquare_eigen(
    number_to_be_squared: U256,
    task_created_block: u32,
    quorum_numbers: Bytes,
    quorum_threshold_percentage: u8,
) -> Result<respondToTaskCall, Infallible> {
    info!(
        "Received job to square the number: {:?}",
        number_to_be_squared
    );
    // Calculate our response to job
    let number_squared = number_to_be_squared.saturating_pow(U256::from(2u32));
    let number_squared = U256::from(4u64);
    let task_response = TaskResponse {
        referenceTaskIndex: task_created_block,
        numberSquared: number_squared,
    };

    let provider = alloy_provider::ProviderBuilder::new()
        .with_recommended_fillers()
        .on_http(EIGENLAYER_HTTP_ENDPOINT)
        .root()
        .clone()
        .boxed();
    let salt: FixedBytes<32> = FixedBytes::from([0x02; 32]);
    let quorum_threshold_percentages: eigensdk::types::operator::QuorumThresholdPercentages =
        vec![eigensdk::types::operator::QuorumThresholdPercentage::from(
            quorum_threshold_percentage,
        )];

    let signer = eigensdk::signer::signer::Config::signer_from_config(
        eigensdk::signer::signer::Config::PrivateKey(PRIVATE_KEY.to_string()),
    )
    .unwrap();
    let bls_key_pair = BlsKeyPair::new(
        "1371012690269088913462269866874713266643928125698382731338806296762673180359922"
            .to_string(),
    )
    .unwrap(); // .map_err(|e| eyre!(e))?;

    // let operator_id = alloy_primitives::FixedBytes(eigensdk::types::operator::operator_id_from_g1_pub_key(bls_key_pair.public_key()).unwrap());
    let operator_id =
        hex!("fd329fe7e54f459b9c104064efe0172db113a50b5f394949b4ef80b3c34ca7f5").into();

    info!("Operator ID: {:?}", operator_id);

    // Create avs clients to interact with contracts deployed on anvil
    let avs_registry_reader = AvsRegistryChainReader::new(
        get_test_logger(),
        *REGISTRY_COORDINATOR_ADDRESS,
        *OPERATOR_STATE_RETRIEVER_ADDRESS,
        EIGENLAYER_HTTP_ENDPOINT.to_string(),
    )
    .await
    .unwrap();
    let avs_writer = AvsRegistryChainWriter::build_avs_registry_chain_writer(
        get_test_logger(),
        http_endpoint.clone(),
        private_key.to_string(),
        registry_coordinator_address,
        operator_state_retriever_address,
    )
    .await
    .unwrap();

    let operators_info = operatorsinfo_inmemory::OperatorInfoServiceInMemory::new(
        get_test_logger(),
        avs_registry_reader.clone(),
        ws_endpoint,
    )
    .await;

    let current_block = provider.get_block_number().await.unwrap();

    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let operators_info_clone = operators_info.clone();
    let token_clone = cancellation_token.clone();
    tokio::task::spawn(async move {
        operators_info_clone
            .start_service(&token_clone, 0, current_block)
            .await
    });
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // Create aggregation service
    let avs_registry_service = chaincaller::AvsRegistryServiceChainCaller::new(
        avs_registry_reader.clone(),
        operators_info,
    );

    // Create an instance of the BLS Aggregator Service
    let bls_agg_service = bls_agg::BlsAggregatorService::new(avs_registry_service);
    let current_block_num = provider.get_block_number().await.unwrap();

    // Create the task related parameters
    // TODO: We need to fetch the index for cases where multiple tasks are created at the same block
    let task_index: eigensdk::types::avs::TaskIndex = 0;
    let time_to_expiry = std::time::Duration::from_secs(60);

    // Initialize the task
    bls_agg_service
        .initialize_new_task(
            task_index,
            task_created_block,
            quorum_numbers.to_vec(),
            quorum_threshold_percentages,
            time_to_expiry,
        )
        .await
        .unwrap();

    // Hash the response with Sha256
    let mut hasher = sha2::Sha256::new();
    let number_squared_bytes = number_squared.to_be_bytes::<32>();
    hasher.update(number_squared_bytes);
    let task_response_digest = alloy_primitives::B256::from_slice(hasher.finalize().as_ref());

    // Hash the response with Keccak256
    let mut keccak_hasher = Keccak256::new();
    keccak_hasher.update(number_squared_bytes);
    let task_response_digest =
        alloy_primitives::B256::from_slice(keccak_hasher.finalize().as_ref());

    let encoded_response = IncredibleSquaringTaskManager::TaskResponse::abi_encode(&task_response);
    let task_response_digest = keccak256(encoded_response);

    // Sign the Hashed Message and send it to the BLS Aggregator
    let bls_signature = bls_key_pair.sign_message(task_response_digest.as_ref());

    let g2_gen = G2Affine::generator();
    let msg_affine = map_to_curve(task_response_digest.as_ref());
    let msg_point = convert_to_g1_point(msg_affine).unwrap();
    let neg_sig = bls_signature.clone().g1_point().g1().neg();

    use ark_ec::pairing::Pairing;
    use std::ops::Neg;
    let e1 = ark_bn254::Bn254::pairing(bls_signature.g1_point().g1(), G2Affine::generator());
    let e2 = ark_bn254::Bn254::pairing(msg_affine, bls_key_pair.public_key_g2().g2());
    assert_eq!(e1, e2);
    info!("Signature is valid: Pairings are equal");

    bls_agg_service
        .process_new_signature(
            task_index,
            task_response_digest,
            bls_signature.clone(),
            operator_id,
        )
        .await
        .unwrap();

    // Wait for the response from the aggregation service
    let bls_agg_response = bls_agg_service
        .aggregated_response_receiver
        .lock()
        .await
        .recv()
        .await
        .unwrap()
        .unwrap();

    // assert_eq!(bls_agg_response.non_signers_pub_keys_g1, vec![]);
    // assert_eq!(bls_agg_response.non_signer_quorum_bitmap_indices, vec![]);
    // assert_eq!(bls_agg_response.total_stake_indices, vec![]);
    // assert_eq!(bls_agg_response.quorum_apk_indices, vec![]);
    // assert_eq!(bls_agg_response.non_signer_stake_indices, vec![]);
    assert_eq!(bls_agg_response.signers_agg_sig_g1, bls_signature.clone());
    assert_eq!(
        bls_agg_response.signers_apk_g2,
        bls_key_pair.public_key_g2()
    );
    assert_eq!(
        bls_agg_response.quorum_apks_g1,
        vec![bls_key_pair.public_key()]
    );

    // Unpack the response and build the NonSignerStakesAndSignature for the response call
    let non_signer_pubkeys = bls_agg_response
        .non_signers_pub_keys_g1
        .into_iter()
        .map(|pubkey| convert_to_g1_point(pubkey.g1()).unwrap())
        .collect();
    let quorum_apks = bls_agg_response
        .quorum_apks_g1
        .into_iter()
        .map(|apk| convert_to_g1_point(apk.g1()).unwrap())
        .collect();
    let non_signer_stakes_and_signature: NonSignerStakesAndSignature =
        NonSignerStakesAndSignature {
            nonSignerQuorumBitmapIndices: bls_agg_response.non_signer_quorum_bitmap_indices,
            nonSignerPubkeys: non_signer_pubkeys,
            nonSignerStakeIndices: bls_agg_response.non_signer_stake_indices,
            quorumApks: quorum_apks,
            apkG2: convert_to_g2_point(bls_agg_response.signers_apk_g2.g2()).unwrap(),
            sigma: convert_to_g1_point(bls_agg_response.signers_agg_sig_g1.g1_point().g1())
                .unwrap(),
            quorumApkIndices: bls_agg_response.quorum_apk_indices,
            totalStakeIndices: bls_agg_response.total_stake_indices,
        };

    // Create respondToTaskCall and encode it
    let call = respondToTaskCall {
        task: Task {
            numberToBeSquared: number_to_be_squared,
            taskCreatedBlock: task_created_block,
            quorumNumbers: quorum_numbers.clone(),
            quorumThresholdPercentage: quorum_threshold_percentage as u32,
        },
        taskResponse: TaskResponse {
            referenceTaskIndex: task_index,
            numberSquared: number_squared,
        },
        nonSignerStakesAndSignature: non_signer_stakes_and_signature.clone(),
    };
    let call_data = call.abi_encode();

    let task_manager_address =
        std::env::var("TASK_MANAGER_ADDR").expect("TASK_MANAGER_ADDR env var is not set");
    let task_manager_address = Address::from_str(&task_manager_address).unwrap();
    let task_manager = IncredibleSquaringTaskManager::new(task_manager_address, provider.clone());
    let receipt = task_manager
        .respondToTask(
            Task {
                numberToBeSquared: number_to_be_squared,
                taskCreatedBlock: task_created_block,
                quorumNumbers: quorum_numbers,
                quorumThresholdPercentage: quorum_threshold_percentage as u32,
            },
            TaskResponse {
                referenceTaskIndex: task_index,
                numberSquared: number_squared,
            },
            non_signer_stakes_and_signature,
        )
        .from(operator_address)
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();

    info!("SUBMITTED DIRECT JOB RESULT: {:?}", receipt);

    assert!(receipt.status());

    // Transaction Manager
    let tx_manager =
        SimpleTxManager::new(get_test_logger(), 1.0, private_key, http_endpoint.as_str()).unwrap();

    let chain_id = provider.get_chain_id().await.unwrap();

    let nonce = provider
        .get_transaction_count(operator_address)
        .await
        .unwrap();

    let operator_address = provider.get_accounts().await.unwrap()[0];

    let nonce = provider
        .get_transaction_count(operator_address)
        .await
        .unwrap();

    let tx = TransactionRequest::default()
        .transaction_type(2) // For EIP-1559 use 2 - For Legacy use 0
        .with_to(task_manager_address)
        .with_from(operator_address)
        .with_value(U256::from(1_000_000_000))
        .with_input(call_data.to_vec())
        .with_nonce(nonce)
        .with_gas_limit(2_000_000)
        .with_chain_id(chain_id)
        .with_gas_price(21_000_000_000);

    let mut tx_request: TransactionRequest = tx.clone();
    let receipt = tx_manager.send_tx(&mut tx_request).await.unwrap();

    info!("JOB SUBMISSION RECEIPT: {:?}", receipt);

    // let signed_tx = signer.sign_transaction_sync(&tx).unwrap();
    // let tx = provider.send_transaction(signed_tx).await.unwrap();
    // // let tx = provider.send_raw_transaction(&call_data).await.unwrap();
    // let receipt = tx.get_receipt().await.unwrap();
    // info!("SUBMITTED JOB RESULT: {:?}", receipt);

    // Send the shutdown signal to the OperatorInfoServiceInMemory
    cancellation_token.cancel();

    // We submit the full task response
    Ok(call)
}

/// Converts the event to inputs.
///
/// Uses a tuple to represent the return type because
/// the macro will index all values in the #[job] function
/// and parse the return type by the index.
pub fn convert_event_to_inputs(
    event: IncredibleSquaringTaskManager::NewTaskCreated,
) -> (U256, u32, Bytes, u8) {
    let number_to_be_squared = event.task.numberToBeSquared;
    let task_created_block = event.task.taskCreatedBlock;
    let quorum_numbers = event.task.quorumNumbers;
    let quorum_threshold_percentage = event.task.quorumThresholdPercentage.try_into().unwrap();
    (
        number_to_be_squared,
        task_created_block,
        quorum_numbers,
        quorum_threshold_percentage,
    )
}

/// Helper for converting a PrimeField to its U256 representation for Ethereum compatibility
/// (U256 reads data as big endian)
pub fn point_to_u256(point: Fq) -> U256 {
    let point = point.into_bigint();
    let point_bytes = point.to_bytes_be();
    U256::from_be_slice(&point_bytes[..])
}

/// Convert [`G1Affine`](ark_bn254::G1Affine) to [`G1Point`](IncredibleSquaringTaskManager::G1Point)
pub fn convert_to_g1_point(
    g1: ark_bn254::G1Affine,
) -> Result<IncredibleSquaringTaskManager::G1Point> {
    let x_point_result = g1.x();
    let y_point_result = g1.y();

    let (Some(&x_point), Some(&y_point)) = (x_point_result, y_point_result) else {
        return Err(eyre!("Invalid G1Affine"));
    };

    let x = BigInt::new(x_point.into_bigint().0);
    let y = BigInt::new(y_point.into_bigint().0);

    let x_u256 = U256::from_limbs(x.0);
    let y_u256 = U256::from_limbs(y.0);

    // Reconstruct the point to ensure it is correct
    let x_bytes: [u8; 32] = x_u256.to_le_bytes();
    let y_bytes: [u8; 32] = y_u256.to_le_bytes();
    let g1_reconstructed = ark_bn254::G1Affine::new(
        Fq::from_le_bytes_mod_order(&x_bytes),
        Fq::from_le_bytes_mod_order(&y_bytes),
    );
    assert_eq!(g1.x().unwrap().0, g1_reconstructed.x().unwrap().0);
    assert_eq!(g1.y().unwrap().0, g1_reconstructed.y().unwrap().0);

    Ok(IncredibleSquaringTaskManager::G1Point {
        X: x_u256,
        Y: y_u256,
    })
}

/// Convert [`G2Affine`] to [`G2Point`]
pub fn convert_to_g2_point(
    g2: ark_bn254::G2Affine,
) -> Result<IncredibleSquaringTaskManager::G2Point> {
    let x_point_result = g2.x();

    let y_point_result = g2.y();

    let (Some(&x_point), Some(&y_point)) = (x_point_result, y_point_result) else {
        return Err(eyre!("Invalid G2Affine"));
    };

    let x_point_c0 = x_point.c0;
    let x_point_c1 = x_point.c1;
    let y_point_c0 = y_point.c0;
    let y_point_c1 = y_point.c1;

    let x_0 = BigInt::new(x_point_c0.into_bigint().0);
    let x_1 = BigInt::new(x_point_c1.into_bigint().0);
    let y_0 = BigInt::new(y_point_c0.into_bigint().0);
    let y_1 = BigInt::new(y_point_c1.into_bigint().0);

    let x_u256_0 = U256::from_limbs(x_0.0);
    let x_u256_1 = U256::from_limbs(x_1.0);
    let y_u256_0 = U256::from_limbs(y_0.0);
    let y_u256_1 = U256::from_limbs(y_1.0);

    Ok(IncredibleSquaringTaskManager::G2Point {
        X: [x_u256_1, x_u256_0],
        Y: [y_u256_1, y_u256_0],
    })
}
