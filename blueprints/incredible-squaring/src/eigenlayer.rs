mod eigenlayer {
    use alloy_network::EthereumWallet;
    use alloy_primitives::{Address, FixedBytes};
    use alloy_provider::ProviderBuilder;
    use alloy_signer::k256::ecdsa::SigningKey;
    use alloy_signer_local::PrivateKeySigner;
    use color_eyre::{eyre::eyre, eyre::OptionExt, Result};
    use gadget_sdk::{
        env::Protocol,
        events_watcher::{
            evm::{Config, EventWatcher},
            substrate::SubstrateEventWatcher,
            tangle::TangleEventsWatcher,
        },
        keystore::Backend,
        network::{
            gossip::NetworkService,
            setup::{start_p2p_network, NetworkConfig},
        },
        tangle_subxt::tangle_testnet_runtime::api::{
            self,
            runtime_types::{sp_core::ecdsa, tangle_primitives::services},
        },
        tx,
    };
    use std::net::IpAddr;

    use eigensdk_rs::eigen_utils::*;
    use eigensdk_rs::incredible_squaring_avs::operator::*;
    use eigensdk_rs::incredible_squaring_avs::*;

    use incredible_squaring_blueprint::{self as blueprint, IncredibleSquaringTaskManager};

    use alloy_signer::k256::PublicKey;
    use gadget_common::config::DebugLogger;
    use gadget_sdk::env::{ContextConfig, GadgetConfiguration};
    use gadget_sdk::events_watcher::tangle::TangleConfig;
    use gadget_sdk::keystore::BackendExt;
    use gadget_sdk::network::gossip::GossipHandle;
    use gadget_sdk::tangle_subxt::subxt;
    use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::sp_core::ed25519;
    use lock_api::{GuardSend, RwLock};
    use sp_core::Pair;
    use std::sync::Arc;

    struct EigenlayerGadgetRunner<R: lock_api::RawRwLock> {
        env: GadgetConfiguration<R>,
        /// The EigenLayer Operator that registers to the AVS and completes the Squaring tasks
        operator: Operator<NodeConfig, OperatorInfoService>,
    }

    struct EigenlayerEventWatcher<T> {
        _phantom: std::marker::PhantomData<T>,
    }

    impl<T: Config> EventWatcher<T> for EigenlayerEventWatcher<T> {
        const TAG: &'static str = "eigenlayer";
        type Contract =
            IncredibleSquaringTaskManager::IncredibleSquaringTaskManagerInstance<T::T, T::P, T::N>;
        type Event = IncredibleSquaringTaskManager::NewTaskCreated;
        const GENESIS_TX_HASH: FixedBytes<32> = FixedBytes::from_slice(&[0; 32]);
    }

    #[async_trait::async_trait]
    impl GadgetRunner for EigenlayerGadgetRunner<parking_lot::RawRwLock> {
        async fn register(&self) -> Result<()> {
            if self.env.test_mode {
                self.env.logger.info("Skipping registration in test mode");
                return Ok(());
            }
            // TODO: We need to support BLS254 in addition to BLS381 in our keystore

            // TODO: Register to become an operator
            let keystore = self.env.keystore().map_err(|e| eyre!(e))?;
            let ecdsa_keypair = self.env.first_ecdsa_signer().map_err(|e| eyre!(e))?;
            let ecdsa_key = keystore
                .iter_ecdsa()
                .next()
                .ok_or_eyre("Unable to find ECDSA key")?;
            let ecdsa_secret_key = keystore
                .expose_ecdsa_secret(&ecdsa_key)
                .map_err(|e| eyre!(e))?
                .ok_or_eyre("Unable to expose ECDSA secret")?;
            let ecdsa_signing_key =
                SigningKey::from_slice(ecdsa_secret_key.to_bytes()).map_err(|e| eyre!(e))?;
            let signer = PrivateKeySigner::from_signing_key(ecdsa_signing_key);
            let wallet = EthereumWallet::from(signer.clone());
            // Implement Eigenlayer-specific registration logic here
            // For example:
            // let contract = YourEigenlayerContract::new(contract_address, provider);
            // contract.register_operator(signer).await?;

            // TODO: Placeholder code for testing - should be retrieved from user
            let http_endpoint = "http://127.0.0.1:8545";
            let ws_endpoint = "ws://127.0.0.1:8545";
            let node_config = NodeConfig {
                node_api_ip_port_address: "127.0.0.1:9808".to_string(),
                eth_rpc_url: http_endpoint.to_string(),
                eth_ws_url: ws_endpoint.to_string(),
                bls_private_key_store_path: "./keystore/bls".to_string(),
                ecdsa_private_key_store_path: "./keystore/ecdsa".to_string(),
                incredible_squaring_service_manager_addr: contract_addresses
                    .service_manager
                    .to_string(),
                avs_registry_coordinator_addr: contract_addresses.registry_coordinator.to_string(),
                operator_state_retriever_addr: contract_addresses
                    .operator_state_retriever
                    .to_string(),
                eigen_metrics_ip_port_address: "127.0.0.1:9100".to_string(),
                delegation_manager_addr: contract_addresses.delegation_manager.to_string(),
                avs_directory_addr: contract_addresses.avs_directory.to_string(),
                operator_address: contract_addresses.operator.to_string(),
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
                .on_http(self.env.rpc_endpoint.parse()?);

            let ws_provider = ProviderBuilder::new()
                .with_recommended_fillers()
                .wallet(wallet)
                .on_http(self.env.rpc_endpoint.parse()?);

            let operator_info_service = OperatorInfoService::new(
                types::OperatorInfo {
                    socket: "0.0.0.0:0".to_string(),
                    pubkeys: operator_pubkeys,
                },
                operator_id,
                signer.address(),
                node_config.clone(),
            );

            // This creates and registers an operator with the given configuration
            let operator = Operator::<NodeConfig, OperatorInfoService>::new_from_config(
                node_config.clone(),
                EigenGadgetProvider {
                    provider: http_provider,
                },
                EigenGadgetProvider {
                    provider: ws_provider,
                },
                operator_info_service,
                signer,
            )
            .await
            .map_err(|e| eyre!(e))?;

            tracing::info!("Registered operator for Eigenlayer");
            Ok(())
        }

        async fn run(&self) -> Result<()> {
            let config = ContextConfig {
                bind_ip: self.env.bind_addr,
                bind_port: self.env.bind_port,
                test_mode: self.env.test_mode,
                logger: self.env.logger.clone(),
            };

            let env =
                gadget_sdk::env::load(Some(Protocol::Eigenlayer), config).map_err(|e| eyre!(e))?;
            let keystore = env.keystore().map_err(|e| eyre!(e))?;
            // get the first ECDSA key from the keystore and register with it.
            let ecdsa_key = self.env.first_ecdsa_signer().map_err(|e| eyre!(e))?;
            let ed_key = keystore
                .iter_ed25519()
                .next()
                .ok_or_eyre("Unable to find ED25519 key")?;
            let ed_public_bytes = ed_key.as_ref(); // 32 byte len

            let ed_public = ed25519_zebra::VerificationKey::try_from(ed_public_bytes)
                .map_err(|e| eyre!("Unable to create ed25519 public key"))?;

            let signing_key = SigningKey::from(ecdsa_key.public_key());
            let priv_key_signer: PrivateKeySigner = PrivateKeySigner::from_signing_key(signing_key);
            let wallet = EthereumWallet::from(priv_key_signer.clone());
            // Set up eignelayer AVS
            let contract_address = Address::from_slice(&[0; 20]);
            // Set up the HTTP provider with the `reqwest` crate.
            let provider = ProviderBuilder::new()
                .with_recommended_fillers()
                .wallet(wallet)
                .on_http(env.rpc_endpoint.parse()?);

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

            let network: GossipHandle = start_p2p_network(network_config).map_err(|e| eyre!(e))?;
            // let x_square_eigen = blueprint::XsquareEigenEventHandler {
            //     ctx: blueprint::MyContext { network, keystore },
            // };
            //
            // let contract: IncredibleSquaringTaskManager::IncredibleSquaringTaskManagerInstance = IncredibleSquaringTaskManager::IncredibleSquaringTaskManagerInstance::new(contract_address, provider);
            //
            // EventWatcher::run(
            //     &EigenlayerEventWatcher,
            //     contract,
            //     // Add more handler here if we have more functions.
            //     vec![Box::new(x_square_eigen)],
            // )
            // .await?;

            Ok(())
        }
    }
}
