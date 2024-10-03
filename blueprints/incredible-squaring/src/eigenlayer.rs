#![allow(dead_code)]
use std::str::FromStr;
use std::{convert::Infallible, ops::Deref, sync::OnceLock};

use alloy_contract::ContractInstance;
use alloy_network::Ethereum;
use alloy_primitives::{address, Address, Bytes, FixedBytes, U256};
use alloy_provider::fillers::{ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller};
use alloy_provider::RootProvider;
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_types::{private::alloy_json_abi::JsonAbi, sol};
use alloy_transport_http::{Client, Http};

use color_eyre::{
    eyre::{eyre, OptionExt},
    Result,
};

use eigensdk::client_avsregistry::writer::AvsRegistryChainWriter;
use eigensdk::client_elcontracts::{
    reader::ELChainReader,
    writer::{ELChainWriter, Operator},
};
use eigensdk::crypto_bls::BlsKeyPair;
use eigensdk::logging::get_test_logger;

use gadget_sdk::{
    config::GadgetConfiguration,
    events_watcher::evm::{Config, EventWatcher},
    info, job,
    run::GadgetRunner,
};

use IncredibleSquaringTaskManager::{
    respondToTaskCall, G1Point, G2Point, NonSignerStakesAndSignature, Task, TaskResponse,
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
    quorum_threshold_percentage: u32,
) -> Result<respondToTaskCall, Infallible> {
    // // 1. Calculate the squared number and save the response
    // let my_msg = MyMessage {
    //     xsquare: number_to_be_squared.saturating_pow(U256::from(2u32)),
    //     signature: Signature::new_zero(),
    // };
    //
    // let responses: HashMap<ecdsa::Public, MyMessage> = HashMap::new();
    // responses.insert(ecdsa::Public::new_zero(), my_msg);
    //
    // // 2. Gossip the squared result BLS signature
    // let handle = &ctx.network;
    // let my_msg_json = serde_json::to_string(&my_msg).unwrap();
    // handle.send_message(my_msg_json).await;
    //
    // // 3. Receive gossiped results from peers
    // let quorum_size = quorum_threshold_percentage as usize;
    // let mut rx = handle.rx_from_inbound.lock().await;
    // while let Some(message) = rx.recv().await {
    //     let message: MyMessage = serde_json::from_str(&message).unwrap();
    //     let public_key = ecdsa::Public::new_zero();
    //     responses.insert(public_key, message);
    //
    //     if responses.len() >= quorum_size {
    //         break;
    //     }
    // }
    //
    // // 4. Once we have a quorum, we calculate the non signers/participants and stakes
    // let non_signer_stakes_and_signature: NonSignerStakesAndSignature = {
    //     let mut non_signer_quorum_bitmap_indices: Vec<u32> = Vec::new();
    //     let mut non_signer_pubkeys: Vec<G1Point> = Vec::new();
    //     let mut non_signer_stake_indices: Vec<u32> = Vec::new();
    //     let mut quorum_apks: Vec<G1Point> = Vec::new();
    //     let mut quorum_apk_indices: Vec<u32> = Vec::new();
    //     let mut total_stake_indices: Vec<u32> = Vec::new();
    //
    //     for (index, (public_key, message)) in responses.iter().enumerate() {
    //         if index >= quorum_size {
    //             non_signer_quorum_bitmap_indices.push(index as u32);
    //             non_signer_pubkeys.push(public_key.clone());
    //             non_signer_stake_indices.push(index as u32);
    //         } else {
    //             quorum_apks.push(message.xsquare.into());
    //             quorum_apk_indices.push(index as u32);
    //             total_stake_indices.push(index as u32);
    //         }
    //     }
    //
    //     NonSignerStakesAndSignature {
    //         nonSignerQuorumBitmapIndices: non_signer_quorum_bitmap_indices,
    //         nonSignerPubkeys: non_signer_pubkeys,
    //         nonSignerStakeIndices: non_signer_stake_indices,
    //         quorumApks: quorum_apks,
    //         quorumApkIndices: quorum_apk_indices,
    //         totalStakeIndices: total_stake_indices,
    //         apkG2: G2Point {
    //             X: [U256::ZERO; 2],
    //             Y: [U256::ZERO; 2],
    //         },
    //         sigma: G1Point {
    //             X: U256::ZERO,
    //             Y: U256::ZERO,
    //         },
    //     }
    // };

    let non_signer_stakes_and_signature: NonSignerStakesAndSignature =
        NonSignerStakesAndSignature {
            nonSignerQuorumBitmapIndices: vec![], // Vec<u32>,
            nonSignerPubkeys: vec![],             // Vec<<G1Point as SolType>::RustType>,
            nonSignerStakeIndices: vec![],        // Vec<u32>,
            quorumApks: vec![],                   // Vec<<G1Point as SolType>::RustType>,
            apkG2: G2Point {
                X: [U256::ZERO; 2], // <G2Point as SolType>::RustType,
                Y: [U256::ZERO; 2], // <G2Point as SolType>::RustType,
            },
            sigma: G1Point {
                X: U256::ZERO, // <G1Point as SolType>::RustType,
                Y: U256::ZERO, // <G1Point as SolType>::RustType,
            },
            quorumApkIndices: vec![],  // Vec<u32>,
            totalStakeIndices: vec![], // Vec<u32>,
        };

    let number_squared = number_to_be_squared.saturating_pow(U256::from(2u32));

    info!("NOW RESPONDING WITH RESULT: {:?}", number_squared);

    // 5. We submit the full task response
    Ok(respondToTaskCall {
        task: Task {
            numberToBeSquared: number_to_be_squared,
            taskCreatedBlock: task_created_block,
            quorumNumbers: quorum_numbers,
            quorumThresholdPercentage: quorum_threshold_percentage,
        },
        taskResponse: TaskResponse {
            referenceTaskIndex: task_created_block,
            numberSquared: number_squared,
        },
        nonSignerStakesAndSignature: non_signer_stakes_and_signature,
    })
}

/// Converts the event to inputs.
///
/// Uses a tuple to represent the return type because
/// the macro will index all values in the #[job] function
/// and parse the return type by the index.
pub fn convert_event_to_inputs(
    event: IncredibleSquaringTaskManager::NewTaskCreated,
) -> (U256, u32, Bytes, u32) {
    let number_to_be_squared = event.task.numberToBeSquared;
    let task_created_block = event.task.taskCreatedBlock;
    let quorum_numbers = event.task.quorumNumbers;
    let quorum_threshold_percentage = event.task.quorumThresholdPercentage;
    (
        number_to_be_squared,
        task_created_block,
        quorum_numbers,
        quorum_threshold_percentage,
    )
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
        let quorum_nums = Bytes::from([0x01]);

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
                http_endpoint.to_string(), // socket
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
