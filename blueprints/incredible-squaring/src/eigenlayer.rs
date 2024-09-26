#![allow(dead_code)]
use std::{convert::Infallible, ops::Deref, sync::OnceLock};

use alloy_contract::ContractInstance;
use alloy_network::{Ethereum, EthereumWallet};
use alloy_primitives::{Address, Bytes, ChainId, FixedBytes, U256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_signer::k256::{ecdsa::SigningKey, elliptic_curve::SecretKey};
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_types::{private::alloy_json_abi::JsonAbi, sol};

use color_eyre::{
    eyre::{eyre, OptionExt},
    Result,
};

use eigensdk_rs::{
    eigen_utils::{
        types::{operator_id_from_key_pair, OperatorPubkeys},
        *,
    },
    incredible_squaring_avs::operator::*,
};

use gadget_sdk::{
    config::GadgetConfiguration,
    events_watcher::evm::{Config, EventWatcher},
    info, job,
    keystore::{sp_core_subxt::Pair as SubxtPair, Backend},
    network::{
        gossip::GossipHandle,
        setup::{start_p2p_network, NetworkConfig},
    },
    run::GadgetRunner,
    tangle_subxt::{
        subxt::tx::Signer,
        tangle_testnet_runtime::api,
        tangle_testnet_runtime::api::runtime_types::{
            sp_core::ecdsa, tangle_primitives::services, tangle_primitives::services::PriceTargets,
        },
    },
    tx,
};

use sp_core::Pair;

use IBLSSignatureChecker::NonSignerStakesAndSignature;
use IIncredibleSquaringTaskManager::{Task, TaskResponse};
use IncredibleSquaringTaskManager::respondToTaskCall;
use BN254::{G1Point, G2Point};

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
            info!("Skipping registration in test mode");
            return Ok(());
        }

        // First we handle the Tangle portion of the Registration
        let client = self.env.client().await.map_err(|e| eyre!(e))?;
        let signer = self.env.first_sr25519_signer().map_err(|e| eyre!(e))?;
        let ecdsa_pair = self.env.first_ecdsa_signer().map_err(|e| eyre!(e))?;
        let xt = api::tx().services().register(
            self.env.blueprint_id,
            services::OperatorPreferences {
                key: ecdsa::Public(ecdsa_pair.signer().public().0),
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
        let result = tx::tangle::send(&client, &signer, &xt).await?;
        info!("Registered operator with hash: {:?}", result);

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
            bls_private_key_store_path: "./../eigensdk-rs/test-utils/keystore/bls".to_string(),
            ecdsa_private_key_store_path: "./../eigensdk-rs/test-utils/keystore/ecdsa".to_string(),
            incredible_squaring_service_manager_addr: "0DCd1Bf9A1b36cE34237eEaFef220932846BCD82"
                .to_string(),
            avs_registry_coordinator_addr: "0B306BF915C4d645ff596e518fAf3F9669b97016".to_string(),
            operator_state_retriever_addr: "3Aa5ebB10DC797CAC828524e59A333d0A371443c".to_string(),
            eigen_metrics_ip_port_address: "127.0.0.1:9100".to_string(),
            delegation_manager_addr: "a85233C63b9Ee964Add6F2cffe00Fd84eb32338f".to_string(),
            avs_directory_addr: "c5a5C42992dECbae36851359345FE25997F5C42d".to_string(),
            operator_address: "f39Fd6e51aad88F6F4ce6aB8827279cffFb92266".to_string(),
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

        info!("Registered operator for Eigenlayer");
        Ok(())
    }

    async fn benchmark(&self) -> std::result::Result<(), Self::Error> {
        todo!()
    }

    async fn run(&mut self) -> Result<()> {
        // Tangle Portion of Run
        let _client = self.env.client().await.map_err(|e| eyre!(e))?;
        let signer = self.env.first_sr25519_signer().map_err(|e| eyre!(e))?;
        info!("Starting the event watcher for {} ...", signer.account_id());

        // Then we handle the EigenLayer portion
        let keystore = self.env.keystore().map_err(|e| eyre!(e))?;

        // TODO: greatly simplify all this logic by using the GenericKeyStore interface
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
        let ecdsa_secret_key_bytes = ecdsa_subxt_key.signer().seed();
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
        };

        let _network: GossipHandle =
            start_p2p_network(network_config).map_err(|e| eyre!(e.to_string()))?;

        let x_square_eigen = XsquareEigenEventHandler {};

        let operator: Operator<NodeConfig, OperatorInfoService> =
            self.operator.clone().ok_or(eyre!("Operator is None"))?;

        let instance =
            IncredibleSquaringTaskManagerInstanceWrapper::new(IncredibleSquaringTaskManager::new(
                operator
                    .incredible_squaring_contract_manager
                    .task_manager_addr,
                operator
                    .incredible_squaring_contract_manager
                    .eth_client_http
                    .clone(),
            ));

        let mut event_watcher: EigenlayerEventWatcher<NodeConfig> = EigenlayerEventWatcher::new();
        event_watcher
            .run(
                instance, vec![Box::new(x_square_eigen)]
            )
            .await?;

        Ok(())
    }
}
