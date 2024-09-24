#![allow(dead_code)]
use alloy_contract::ContractInstance;
use alloy_network::Ethereum;
use alloy_network::EthereumWallet;
use alloy_primitives::{Address, ChainId, FixedBytes};
use alloy_primitives::{Bytes, U256};
use alloy_provider::Provider;
use alloy_provider::ProviderBuilder;
use alloy_signer::k256::ecdsa::SigningKey;
use alloy_signer::k256::elliptic_curve::SecretKey;
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_types::private::alloy_json_abi::JsonAbi;
use alloy_sol_types::sol;
use alloy_transport::Transport;
use color_eyre::{eyre::eyre, eyre::OptionExt, Result};
use gadget_sdk::job;
use gadget_sdk::{
    events_watcher::evm::{Config, EventWatcher},
    keystore::Backend,
    network::setup::{start_p2p_network, NetworkConfig},
    run::GadgetRunner,
};
use std::convert::Infallible;
use std::ops::Deref;
use std::sync::OnceLock;
use IBLSSignatureChecker::NonSignerStakesAndSignature;
use IIncredibleSquaringTaskManager::{Task, TaskResponse};
use IncredibleSquaringTaskManager::respondToTaskCall;
use BN254::{G1Point, G2Point};

use gadget_sdk::{
    tangle_subxt::tangle_testnet_runtime::api::{
        runtime_types::{sp_core::ecdsa, tangle_primitives::services},
    },
    tx,
};
use eigensdk_rs::eigen_utils::types::{operator_id_from_key_pair, OperatorPubkeys};
use eigensdk_rs::eigen_utils::*;
use eigensdk_rs::incredible_squaring_avs::operator::*;
use sp_core::Pair;
use subxt_signer::ecdsa::SecretKeyBytes;
use gadget_sdk::config::GadgetConfiguration;
use gadget_sdk::network::gossip::GossipHandle;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::PriceTargets;

// Codegen from ABI file to interact with the contract.
sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    IncredibleSquaringTaskManager,
    "contracts/out/IncredibleSquaringTaskManager.sol/IncredibleSquaringTaskManager.json"
);

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
    // TODO: Add Context
    // ctx: &MyContext,
    number_to_be_squared: U256,
    task_created_block: u32,
    quorum_numbers: Bytes,
    quorum_threshold_percentage: u32,
) -> Result<respondToTaskCall, Infallible> {
    // TODO: Send our task response to Aggregator RPC server

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
    // let quorum_size = 1;
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
            numberSquared: number_to_be_squared.saturating_pow(U256::from(2u32)),
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

#[async_trait::async_trait]
impl GadgetRunner for EigenlayerGadgetRunner<parking_lot::RawRwLock> {
    type Error = color_eyre::eyre::Report;

    fn config(&self) -> &GadgetConfiguration<parking_lot::RawRwLock> {
        todo!()
    }

    async fn register(&mut self) -> Result<()> {
        if self.env.test_mode {
            self.env.logger.info("Skipping registration in test mode");
            return Ok(());
        }

        // First we handle the Tangle portion of the Registration
        let client = self.env.client().await.map_err(|e| eyre!(e))?;
        let signer = self.env.first_signer().map_err(|e| eyre!(e))?;
        let ecdsa_pair = self.env.first_ecdsa_signer().map_err(|e| eyre!(e))?;
        let xt = api::tx().services().register(
            self.env.blueprint_id,
            services::OperatorPreferences {
                key: ecdsa::Public(ecdsa_pair.public_key().0),
                approval: services::ApprovalPrefrence::None,
                // TODO: Set the price targets
                price_targets: PriceTargets {
                    cpu: 0,
                    mem: 0,
                    storage_hdd: 0,
                    storage_ssd: 0,
                    storage_nvme: 0,
                },
            },
            Default::default(),
        );
        // send the tx to the tangle and exit.
        let result = tx::tangle::send(&client, &signer, &xt, &self.env.logger).await?;
        self.env
            .logger
            .info(format!("Registered operator with hash: {:?}", result));

        // Now we handle the EigenLayer portion of the Registration
        let keystore = self.env.keystore().map_err(|e| eyre!(e))?;
        let bls_keypair = self.env.first_bls_bn254_signer().map_err(|e| eyre!(e))?;

        let _ecdsa_keypair = self.env.first_ecdsa_signer().map_err(|e| eyre!(e))?;
        let ecdsa_key = keystore
            .iter_ecdsa()
            .next()
            .ok_or_eyre("Unable to find ECDSA key")?;
        let ecdsa_secret_key = keystore
            .expose_ecdsa_secret(&ecdsa_key)
            .map_err(|e| eyre!(e))?
            .ok_or_eyre("Unable to expose ECDSA secret")?;
        let ecdsa_signing_key =
            SigningKey::from_slice(&ecdsa_secret_key.to_bytes()).map_err(|e| eyre!(e))?;
        let signer = PrivateKeySigner::from_signing_key(ecdsa_signing_key.clone());
        let wallet = EthereumWallet::from(signer.clone());

        // TODO: Placeholder code for testing - should be retrieved, not hardcoded
        let http_endpoint = "http://127.0.0.1:8545";
        let ws_endpoint = "ws://127.0.0.1:8545";
        let node_config = NodeConfig {
            node_api_ip_port_address: "127.0.0.1:9808".to_string(),
            eth_rpc_url: http_endpoint.to_string(),
            eth_ws_url: ws_endpoint.to_string(),
            bls_private_key_store_path: "./keystore/bls".to_string(),
            ecdsa_private_key_store_path: "./keystore/ecdsa".to_string(),
            incredible_squaring_service_manager_addr: Default::default(),
            avs_registry_coordinator_addr: Default::default(),
            operator_state_retriever_addr: Default::default(),
            eigen_metrics_ip_port_address: "127.0.0.1:9100".to_string(),
            delegation_manager_addr: Default::default(),
            avs_directory_addr: Default::default(),
            operator_address: Default::default(),
            enable_metrics: false,
            enable_node_api: false,
            server_ip_port_address: "127.0.0.1:8673".to_string(),
            metadata_url:
                "https://github.com/webb-tools/eigensdk-rs/blob/main/test-utils/metadata.json"
                    .to_string(),
        };

        let http_provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet.clone())
            .on_http(self.env.rpc_endpoint.parse()?)
            .root()
            .clone()
            .boxed();

        let ws_provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_http(self.env.rpc_endpoint.parse()?)
            .root()
            .clone()
            .boxed();

        let operator_info_service = OperatorInfoService::new(
            types::OperatorInfo {
                socket: "0.0.0.0:0".to_string(),
                pubkeys: OperatorPubkeys {
                    g1_pubkey: bls_keypair.get_pub_key_g1().to_ark_g1(),
                    g2_pubkey: bls_keypair.get_pub_key_g2().to_ark_g2(),
                },
            },
            operator_id_from_key_pair(&bls_keypair),
            signer.address(),
            node_config.clone(),
        );

        let chain_id = http_provider
            .get_chain_id()
            .await
            .map_err(|e| OperatorError::HttpEthClientError(e.to_string()))?;

        let signer = EigenGadgetSigner::new(
            PrivateKeySigner::from_signing_key(ecdsa_signing_key),
            Some(ChainId::from(chain_id)),
        );

        // This creates and registers an operator with the given configuration
        let operator = Operator::<NodeConfig, OperatorInfoService>::new_from_config(
            node_config.clone(),
            EigenGadgetProvider {
                provider: http_provider.clone(),
            },
            EigenGadgetProvider {
                provider: ws_provider.clone(),
            },
            operator_info_service.clone(),
            signer.clone(),
        )
        .await
        .map_err(|e| eyre!(e))?;

        self.set_operator(operator);

        tracing::info!("Registered operator for Eigenlayer");
        Ok(())
    }

    async fn benchmark(&self) -> std::result::Result<(), Self::Error> {
        todo!()
    }

    async fn run(&mut self) -> Result<()> {
        // Tangle Portion of Run
        let _client = self.env.client().await.map_err(|e| eyre!(e))?;
        let signer = self.env.first_signer().map_err(|e| eyre!(e))?;
        let _logger = self.env.logger.clone();
        self.env.logger.info(format!(
            "Starting the event watcher for {} ...",
            signer.account_id()
        ));

        // Then we handle the EigenLayer portion
        let keystore = self.env.keystore().map_err(|e| eyre!(e))?;

        // ED25519 Key Retrieval
        let ed_key = keystore
            .iter_ed25519()
            .next()
            .ok_or_eyre("Unable to find ED25519 key")?;
        let ed_public_bytes = ed_key.as_ref(); // 32 byte len
        let ed_public = ed25519_zebra::VerificationKey::try_from(ed_public_bytes)
            .map_err(|_| eyre!("Unable to create ed25519 public key"))?;

        // ECDSA Key Retrieval
        let ecdsa_subxt_key = self.env.first_ecdsa_signer().map_err(|e| eyre!(e))?;
        let ecdsa_secret_key_bytes =
            SecretKeyBytes::from(ecdsa_subxt_key.0.secret_key().secret_bytes());
        let ecdsa_secret_key =
            SecretKey::from_slice(&ecdsa_secret_key_bytes).map_err(|e| eyre!(e))?;
        let ecdsa_signing_key = SigningKey::from(&ecdsa_secret_key);
        let ecdsa_key =
            sp_core::ecdsa::Pair::from_seed_slice(&ecdsa_secret_key_bytes).map_err(|e| eyre!(e))?;

        // Construct Signer
        let priv_key_signer: PrivateKeySigner =
            PrivateKeySigner::from_signing_key(ecdsa_signing_key);
        let wallet = EthereumWallet::from(priv_key_signer.clone());

        // Set up eignelayer AVS
        let _contract_address = Address::from_slice(&[0; 20]);
        // Set up the HTTP provider with the `reqwest` crate.
        let _provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_http(self.env.rpc_endpoint.parse()?);

        let sr_secret = keystore
            .expose_ed25519_secret(&ed_public)
            .map_err(|e| eyre!(e))?
            .ok_or_eyre("Unable to find ED25519 secret")?;
        let mut sr_secret_bytes = sr_secret.as_ref().to_vec(); // 64 byte len

        let identity = libp2p::identity::Keypair::ed25519_from_bytes(&mut sr_secret_bytes)
            .map_err(|e| eyre!("Unable to construct libp2p keypair: {e:?}"))?;

        // TODO: Fill in and find the correct values for the network configuration
        // TODO: Implementations for reading set of operators from Tangle & Eigenlayer
        let network_config: NetworkConfig = NetworkConfig {
            identity,
            ecdsa_key,
            bootnodes: vec![],
            bind_ip: self.env.bind_addr,
            bind_port: self.env.bind_port,
            topics: vec!["__TESTING_INCREDIBLE_SQUARING".to_string()],
            logger: self.env.logger.clone(),
        };

        let _network: GossipHandle =
            start_p2p_network(network_config).map_err(|e| eyre!(e.to_string()))?;

        // let x_square_eigen = blueprint::XsquareEigenEventHandler {
        //     ctx: blueprint::MyContext { network, keystore },
        // };

        //
        // EventWatcher::run(
        //     &EigenlayerEventWatcher,
        //     contract,
        //     // Add more handler here if we have more functions.
        //     vec![Box::new(x_square_eigen)],
        // )
        // .await?;'

        let operator = self.operator.ok_or(eyre!("Run Error - No Operator Found"))?;

        let instance = IncredibleSquaringTaskManagerInstanceWrapper::new(
            IncredibleSquaringTaskManager::new(
                operator.incredible_squaring_contract_manager.task_manager_addr.clone(),
                operator.incredible_squaring_contract_manager.eth_client_http.clone(),
            ),
        );

        let event_watcher: EigenlayerEventWatcher<NodeConfig> = EigenlayerEventWatcher::new();
        event_watcher
            .run(
                instance,
                vec![
                // Box::new(x_square_eigen)
                ],
            )
            .await?;

        // self.operator.unwrap().start().await?;
        // if let Some(operator) = &self.operator {
        //     operator.start().await?;
        // } else {
        //     return Err(eyre!("Run Error - No Operator Found"));
        // }

        Ok(())
    }
}
