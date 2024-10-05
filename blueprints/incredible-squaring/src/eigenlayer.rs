#![allow(dead_code)]
use std::str::FromStr;
use std::{convert::Infallible, ops::Deref, sync::OnceLock};

use alloy_contract::ContractInstance;
use alloy_network::Ethereum;
use alloy_network::TransactionBuilder;
use alloy_primitives::{address, hex, Address, Bytes, FixedBytes, U256};
use alloy_provider::fillers::{ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller};
use alloy_provider::Provider;
use alloy_provider::RootProvider;
use alloy_rpc_types_eth::TransactionRequest;
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_types::SolCall;
use alloy_sol_types::{private::alloy_json_abi::JsonAbi, sol};
use alloy_transport_http::{Client, Http};
use ark_ec::AffineRepr;
use ark_ff::PrimeField;
use color_eyre::{
    eyre::{eyre, OptionExt},
    Result,
};
use eigensdk::chainio_txmanager::simple_tx_manager::SimpleTxManager;
use eigensdk::client_avsregistry::reader::AvsRegistryChainReader;
use eigensdk::client_avsregistry::writer::AvsRegistryChainWriter;
use eigensdk::client_elcontracts::{
    reader::ELChainReader,
    writer::{ELChainWriter, Operator},
};
use eigensdk::crypto_bls::BlsKeyPair;
use eigensdk::logging::get_test_logger;
use eigensdk::services_avsregistry::chaincaller;
use eigensdk::services_blsaggregation::bls_agg;
use eigensdk::services_operatorsinfo::operatorsinfo_inmemory;
use gadget_sdk::{
    config::GadgetConfiguration,
    events_watcher::evm::{Config, EventWatcher},
    info, job,
    run::GadgetRunner,
};

use k256::sha2::{self, Digest};
use IncredibleSquaringTaskManager::{
    respondToTaskCall, NonSignerStakesAndSignature, Task, TaskResponse,
};

use serde_json::Value;
use structopt::lazy_static::lazy_static;

lazy_static! {
    /// 1 day
    static ref SIGNATURE_EXPIRY: U256 = U256::from(86400);
}

// Codegen from ABI file to interact with the contract.
sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    IncredibleSquaringTaskManager,
    "contracts/out/IncredibleSquaringTaskManager.sol/IncredibleSquaringTaskManager.json"
);

#[derive(Debug, Clone)]
struct NodeConfig {}

impl Config for NodeConfig {
    type TH = Http<Client>;
    type PH = FillProvider<
        JoinFill<
            JoinFill<JoinFill<alloy_provider::Identity, GasFiller>, NonceFiller>,
            ChainIdFiller,
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
    event_handler(
        protocol = "eigenlayer",
        instance = IncredibleSquaringTaskManager,
        event = IncredibleSquaringTaskManager::NewTaskCreated,
        event_converter = convert_event_to_inputs,
        callback = IncredibleSquaringTaskManager::IncredibleSquaringTaskManagerCalls::respondToTask
    ),
)]
pub async fn xsquare_eigen(
    number_to_be_squared: U256,
    task_created_block: u32,
    quorum_numbers: Bytes,
    quorum_threshold_percentage: u8,
) -> Result<respondToTaskCall, Infallible> {
    // Calculate our response to job
    let number_squared = number_to_be_squared.saturating_pow(U256::from(2u32));
    let task_response = TaskResponse {
        referenceTaskIndex: task_created_block,
        numberSquared: number_squared,
    };

    // TODO: We can make these constants
    let service_manager_address = address!("67d269191c92caf3cd7723f116c85e6e9bf55933");
    let registry_coordinator_address = address!("c3e53f4d16ae77db1c982e75a937b9f60fe63690");
    let operator_state_retriever_address = address!("1613beb3b2c4f22ee086b2b38c1476a3ce7f78e8");
    let operator_address = address!("f39fd6e51aad88f6f4ce6ab8827279cfffb92266");
    let http_endpoint = "http://127.0.0.1:8545".to_string();
    let ws_endpoint = "ws://127.0.0.1:8545".to_string();

    // let provider = eigensdk::utils::get_provider(&http_endpoint);
    let provider = alloy_provider::ProviderBuilder::new()
        .with_recommended_fillers()
        .on_http(http_endpoint.parse().unwrap())
        .root()
        .clone()
        .boxed();
    let salt: FixedBytes<32> = FixedBytes::from([0x02; 32]);
    let quorum_threshold_percentages: eigensdk::types::operator::QuorumThresholdPercentages =
        vec![eigensdk::types::operator::QuorumThresholdPercentage::from(
            quorum_threshold_percentage,
        )];

    let private_key = "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
    let signer = eigensdk::signer::signer::Config::signer_from_config(
        eigensdk::signer::signer::Config::PrivateKey(private_key.to_string()),
    )
    .unwrap();
    let bls_key_pair = BlsKeyPair::new(
        "12248929636257230549931416853095037629726205319386239410403476017439825112537".to_string(),
    )
    .unwrap(); // .map_err(|e| eyre!(e))?;

    // let operator_id = alloy_primitives::FixedBytes(operator_id_from_g1_pub_key(bls_key_pair.public_key()).unwrap());
    let operator_id =
        hex!("b345f720903a3ecfd59f3de456dd9d266c2ce540b05e8c909106962684d9afa3").into();
    info!("Operator ID: {:?}", operator_id);

    // Create avs clients to interact with contracts deployed on anvil
    let avs_registry_reader = AvsRegistryChainReader::new(
        get_test_logger(),
        registry_coordinator_address,
        operator_state_retriever_address,
        http_endpoint.clone(),
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

    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    // Create aggregation service
    let avs_registry_service = chaincaller::AvsRegistryServiceChainCaller::new(
        avs_registry_reader.clone(),
        operators_info,
    );

    let bls_agg_service = bls_agg::BlsAggregatorService::new(avs_registry_service);
    let current_block_num = provider.get_block_number().await.unwrap();

    // Create the task related parameters
    let task_index: eigensdk::types::avs::TaskIndex = task_created_block;
    let time_to_expiry = std::time::Duration::from_secs(60);

    // Initialize the task
    bls_agg_service
        .initialize_new_task(
            task_index,
            current_block_num as u32,
            quorum_numbers.to_vec(),
            quorum_threshold_percentages,
            time_to_expiry,
        )
        .await
        .unwrap();

    // Hash the response
    let mut hasher = sha2::Sha256::new();
    let number_squared_bytes = number_squared.to_be_bytes::<32>();
    hasher.update(number_squared_bytes);
    let task_response_digest = alloy_primitives::B256::from_slice(hasher.finalize().as_ref());

    let bls_signature = bls_key_pair.sign_message(task_response_digest.as_ref());
    bls_agg_service
        .process_new_signature(task_index, task_response_digest, bls_signature, operator_id)
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

    // Send the shutdown signal to the OperatorInfoServiceInMemory
    cancellation_token.cancel();

    let bls_agg::BlsAggregationServiceResponse {
        task_index: _,
        task_response_digest: _,
        non_signers_pub_keys_g1,
        quorum_apks_g1,
        signers_apk_g2,
        signers_agg_sig_g1,
        non_signer_quorum_bitmap_indices,
        quorum_apk_indices,
        total_stake_indices,
        non_signer_stake_indices,
    } = bls_agg_response;

    // Convert to pub keys to expected type
    let pub_keys = non_signers_pub_keys_g1
        .iter()
        .map(|key| key.g1())
        .collect::<Vec<_>>();

    let non_signer_pubkeys = non_signers_pub_keys_g1
        .into_iter()
        .map(|pubkey| convert_to_g1_point(pubkey.g1()).unwrap())
        .collect();

    let quorum_apks = quorum_apks_g1
        .into_iter()
        .map(|apk| convert_to_g1_point(apk.g1()).unwrap())
        .collect();

    let non_signer_stakes_and_signature: NonSignerStakesAndSignature =
        NonSignerStakesAndSignature {
            nonSignerQuorumBitmapIndices: non_signer_quorum_bitmap_indices,
            nonSignerPubkeys: non_signer_pubkeys,
            nonSignerStakeIndices: non_signer_stake_indices,
            quorumApks: quorum_apks,
            apkG2: convert_to_g2_point(signers_apk_g2.g2()).unwrap(),
            sigma: convert_to_g1_point(signers_agg_sig_g1.g1_point().g1()).unwrap(),
            quorumApkIndices: quorum_apk_indices,
            totalStakeIndices: total_stake_indices,
        };

    let number_squared = number_to_be_squared.saturating_pow(U256::from(2u32));

    info!("NOW RESPONDING WITH RESULT: {:?}", number_squared);

    let call = respondToTaskCall {
        task: Task {
            numberToBeSquared: number_to_be_squared,
            taskCreatedBlock: task_created_block,
            quorumNumbers: quorum_numbers.clone(),
            quorumThresholdPercentage: quorum_threshold_percentage as u32,
        },
        taskResponse: TaskResponse {
            referenceTaskIndex: task_created_block,
            numberSquared: number_squared,
        },
        nonSignerStakesAndSignature: non_signer_stakes_and_signature.clone(),
    };

    let call_data = call.abi_encode();
    info!("SUBMITTED JOB: {:?}", call_data);

    let task_manager_address =
        std::env::var("TASK_MANAGER_ADDR").expect("TASK_MANAGER_ADDR env var is not set");
    let task_manager_address = Address::from_str(&task_manager_address).unwrap();

    // let task_manager = IncredibleSquaringTaskManager::new(task_manager_address, provider.clone());
    // let receipt = task_manager.respondToTask(
    //     Task {
    //         numberToBeSquared: number_to_be_squared,
    //         taskCreatedBlock: task_created_block,
    //         quorumNumbers: quorum_numbers,
    //         quorumThresholdPercentage: quorum_threshold_percentage as u32,
    //     },
    //     TaskResponse {
    //         referenceTaskIndex: task_created_block,
    //         numberSquared: number_squared,
    //     },
    //     non_signer_stakes_and_signature,
    // )
    //     .send()
    //     .await
    //     .unwrap()
    //     .get_receipt()
    //     .await
    //     .unwrap();
    //
    // info!("SUBMITTED JOB RESULT: {:?}", receipt);

    // Create a transaction object
    // let tx = alloy_rpc_types::TransactionRequest::new()
    //     .to(task_manager_address)
    //     .data(call_data);

    // Transaction Manager
    let tx_manager =
        SimpleTxManager::new(get_test_logger(), 1.0, private_key, http_endpoint.as_str()).unwrap();

    // let tx = alloy_rpc_types::TransactionRequest {
    //     // to: Some(task_manager_address),
    //     input: alloy_rpc_types::TransactionInput::new(Bytes::from(call_data)),
    //     ..Default::default()
    // };

    let chain_id = provider.get_chain_id().await.unwrap();

    let nonce = provider
        .get_transaction_count(operator_address)
        .await
        .unwrap();

    // ---------------------
    // let mut tx = alloy_consensus::TxLegacy {
    //     to: task_manager_address.into(),
    //     value: U256::from(1_000_000_000),
    //     gas_limit: 2_000_000,
    //     nonce: nonce.clone(),
    //     gas_price: 21_000_000_000,
    //     input: call_data.into(),
    //     chain_id: Some(chain_id),
    // };
    let operator_address = provider.get_accounts().await.unwrap()[0];

    let nonce = provider
        .get_transaction_count(operator_address)
        .await
        .unwrap();

    let tx = TransactionRequest::default()
        .transaction_type(0) // For EIP-1559 use 2 - For Legacy use 0
        .with_to(task_manager_address)
        .with_from(operator_address)
        .with_value(U256::from(1_000_000_000))
        .with_input(call_data.to_vec())
        .with_nonce(nonce)
        .with_gas_limit(2_000_000)
        .with_chain_id(chain_id)
        .with_gas_price(21_000_000_000);

    // let wallet = EthereumWallet::from(signer);
    //
    // let signed_tx = tx.build(&wallet).await.unwrap();
    //
    // let receipt = provider
    //     .send_transaction(signed_tx.clone().into())
    //     .await
    //     .unwrap();
    //     // .get_receipt()
    //     // .await
    //     // .unwrap();
    //
    // info!("SUBMITTED JOB RESULT: {:?}", receipt);
    //
    // let &tx_hash = signed_tx.tx_hash();
    // // Retry waiting for the transaction receipt with backoff
    // let mut attempts = 0;
    // let max_attempts = 10;
    // let mut receipt = None;
    //
    // while attempts < max_attempts {
    //     match provider
    //         .raw_request::<_, Value>(Cow::Borrowed("eth_getTransactionReceipt"), &[tx_hash])
    //         .await
    //     {
    //         Ok(r) => {
    //             if r.is_object() {
    //                 receipt = Some(r);
    //                 break;
    //             } else {
    //                 println!("Transaction not yet mined, retrying...");
    //                 tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    //             }
    //         }
    //         Err(e) => {
    //             println!("Error while waiting for receipt: {:?}", e);
    //             tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    //         }
    //     }
    //     attempts += 1;
    // }
    //
    // if let Some(receipt_json) = receipt {
    //     println!("Transaction mined: {:?}", receipt_json);
    //     // match serde_json::from_value::<alloy_rpc_types_eth::TransactionReceipt>(receipt_json) {
    //     //     Ok(receipt) => println!("Transaction mined: {:?}", receipt),
    //     //     Err(e) => println!("Failed to deserialize receipt: {:?}", e),
    //     // }
    // } else {
    //     println!(
    //         "Failed to get the transaction receipt after {} attempts",
    //         max_attempts
    //     );
    // }

    // let receipt = provider.request("eth_getTransactionReceipt", &[tx_hash]).await.unwrap();

    let mut tx_request: TransactionRequest = tx.clone();
    let receipt = tx_manager.send_tx(&mut tx_request).await.unwrap();

    info!("JOB SUBMISSION RECEIPT: {:?}", receipt);

    // let signed_tx = signer.sign_transaction_sync(&mut tx).unwrap();

    // ---------------------

    // let signed_tx = signer.sign_transaction_sync(&tx).unwrap();

    // let tx = provider.send_transaction(signed_tx).await.unwrap();
    //
    // // let tx = provider.send_raw_transaction(&call_data).await.unwrap();
    // let receipt = tx.get_receipt().await.unwrap();
    // info!("SUBMITTED JOB RESULT: {:?}", receipt);

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

/// Convert [`G1Affine`](ark_bn254::G1Affine) to [`G1Point`](IncredibleSquaringTaskManager::G1Point)
pub fn convert_to_g1_point(
    g1: ark_bn254::G1Affine,
) -> Result<IncredibleSquaringTaskManager::G1Point> {
    let x_point_result = g1.x();
    let y_point_result = g1.y();

    let (Some(x_point), Some(y_point)) = (x_point_result, y_point_result) else {
        return Err(eyre!("Invalid G1Affine"));
    };

    let x = ark_ff::BigInt::new(x_point.into_bigint().0);
    let y = ark_ff::BigInt::new(y_point.into_bigint().0);

    let x_u256 = U256::from_limbs(x.0);
    let y_u256 = U256::from_limbs(y.0);

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
    // let x_point_c1 = g2.x().unwrap().c1;

    let y_point_result = g2.y();
    // let y_point_c1 = g2.y().unwrap().c1;

    let (Some(x_point), Some(y_point)) = (x_point_result, y_point_result) else {
        return Err(eyre!("Invalid G2Affine"));
    };
    let x_point_c0 = x_point.c0;
    let x_point_c1 = x_point.c1;
    let y_point_c0 = y_point.c0;
    let y_point_c1 = y_point.c1;

    let x_0 = ark_ff::BigInt::new(x_point_c0.into_bigint().0);
    let x_1 = ark_ff::BigInt::new(x_point_c1.into_bigint().0);
    let y_0 = ark_ff::BigInt::new(y_point_c0.into_bigint().0);
    let y_1 = ark_ff::BigInt::new(y_point_c1.into_bigint().0);

    let x_u256_0 = U256::from_limbs(x_0.0);
    let x_u256_1 = U256::from_limbs(x_1.0);
    let y_u256_0 = U256::from_limbs(y_0.0);
    let y_u256_1 = U256::from_limbs(y_1.0);

    Ok(IncredibleSquaringTaskManager::G2Point {
        X: [x_u256_1, x_u256_0],
        Y: [y_u256_1, y_u256_0],
    })
}

pub struct EigenlayerGadgetRunner<R: lock_api::RawRwLock> {
    pub env: GadgetConfiguration<R>,
}

impl<R: lock_api::RawRwLock> EigenlayerGadgetRunner<R> {
    pub async fn new(env: GadgetConfiguration<R>) -> Self {
        Self { env }
    }

    pub fn address(&self) -> Option<Address> {
        self.env.contract_address
    }
}

#[async_trait::async_trait]
impl GadgetRunner for EigenlayerGadgetRunner<parking_lot::RawRwLock> {
    type Error = color_eyre::eyre::Report;

    fn config(&self) -> &GadgetConfiguration<parking_lot::RawRwLock> {
        todo!()
    }

    async fn register(&mut self) -> Result<()> {
        if self.env.test_mode {
            info!("Skipping registration in test mode");
            return Ok(());
        }

        let http_endpoint = "http://127.0.0.1:8545";
        let pvt_key = "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

        // TODO: Should be pulled from environment variables
        // let service_manager_address = address!("67d269191c92caf3cd7723f116c85e6e9bf55933");
        let registry_coordinator_address = address!("c3e53f4d16ae77db1c982e75a937b9f60fe63690");
        let operator_state_retriever_address = address!("1613beb3b2c4f22ee086b2b38c1476a3ce7f78e8");
        let delegation_manager_address = address!("dc64a140aa3e981100a9beca4e685f962f0cf6c9");
        let strategy_manager_address = address!("5fc8d32690cc91d4c39d9d3abcbd16989f875707");
        // let erc20_mock_address = address!("7969c5ed335650692bc04293b07f5bf2e7a673c0");
        let avs_directory_address = address!("0000000000000000000000000000000000000000");

        let provider = eigensdk::utils::get_provider(http_endpoint);

        // TODO: Move Slasher retrieval into SDK
        let delegation_manager = eigensdk::utils::binding::DelegationManager::new(
            delegation_manager_address,
            provider.clone(),
        );
        let slasher_address = delegation_manager.slasher().call().await.map(|a| a._0)?;

        let test_logger = get_test_logger();
        let avs_registry_writer = AvsRegistryChainWriter::build_avs_registry_chain_writer(
            test_logger.clone(),
            http_endpoint.to_string(),
            pvt_key.to_string(),
            registry_coordinator_address,
            operator_state_retriever_address,
        )
        .await
        .expect("avs writer build fail ");

        // TODO: Retrieve BLS Secret Key from Keystore
        // Create a new key pair instance using the secret key
        let bls_key_pair = BlsKeyPair::new(
            "12248929636257230549931416853095037629726205319386239410403476017439825112537"
                .to_string(),
        )
        .map_err(|e| eyre!(e))?;

        let digest_hash: FixedBytes<32> = FixedBytes::from([0x02; 32]);

        // Get the current SystemTime
        let now = std::time::SystemTime::now();
        let mut sig_expiry: U256 = U256::from(0);
        // Convert SystemTime to a Duration since the UNIX epoch
        if let Ok(duration_since_epoch) = now.duration_since(std::time::UNIX_EPOCH) {
            // Convert the duration to seconds
            let seconds = duration_since_epoch.as_secs(); // Returns a u64

            // Convert seconds to U256
            sig_expiry = U256::from(seconds) + *SIGNATURE_EXPIRY;
        } else {
            info!("System time seems to be before the UNIX epoch.");
        }
        let quorum_nums = Bytes::from(vec![0]);

        // A new ElChainReader instance
        let el_chain_reader = ELChainReader::new(
            get_test_logger().clone(),
            slasher_address,
            delegation_manager_address,
            avs_directory_address,
            http_endpoint.to_string(),
        );
        // A new ElChainWriter instance
        let el_writer = ELChainWriter::new(
            delegation_manager_address,
            strategy_manager_address,
            el_chain_reader,
            http_endpoint.to_string(),
            pvt_key.to_string(),
        );

        let wallet = PrivateKeySigner::from_str(
            "bead471191bea97fc3aeac36c9d74c895e8a6242602e144e43152f96219e96e8",
        )
        .expect("no key ");

        let operator_details = Operator {
            address: wallet.address(),
            earnings_receiver_address: wallet.address(),
            delegation_approver_address: wallet.address(),
            staker_opt_out_window_blocks: 3,
            metadata_url: Some(
                "https://github.com/webb-tools/eigensdk-rs/blob/main/test-utils/metadata.json"
                    .to_string(),
            ), // TODO: Metadata should be from Environment Variable
        };

        // Register the address as operator in delegation manager
        el_writer
            .register_as_operator(operator_details)
            .await
            .map_err(|e| eyre!(e))?;

        // Register the operator in registry coordinator
        avs_registry_writer
            .register_operator_in_quorum_with_avs_registry_coordinator(
                bls_key_pair,
                digest_hash,
                sig_expiry,
                quorum_nums,
                http_endpoint.to_string(),
            )
            .await?;

        info!("Registered operator for Eigenlayer");
        Ok(())
    }

    async fn benchmark(&self) -> std::result::Result<(), Self::Error> {
        todo!()
    }

    async fn run(&mut self) -> Result<()> {
        let contract_address = self.address().ok_or_eyre("Contract address not set")?;
        let http_endpoint = "http://127.0.0.1:8545";
        let provider = eigensdk::utils::get_provider(http_endpoint);

        let mut event_watcher: EigenlayerEventWatcher<NodeConfig> =
            EigenlayerEventWatcher::new(contract_address, provider.clone());
        event_watcher.run().await?;

        Ok(())
    }
}
