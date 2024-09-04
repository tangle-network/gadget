use alloy_network::EthereumWallet;
use alloy_primitives::{Address, FixedBytes};
use alloy_provider::ProviderBuilder;
use alloy_signer::k256::ecdsa::SigningKey;
use alloy_signer_local::PrivateKeySigner;
use color_eyre::{eyre, eyre::eyre, eyre::OptionExt, Result};
use gadget_sdk::{
    env::{Protocol, StdGadgetConfiguration},
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
    // run::GadgetRunner,
    tangle_subxt::tangle_testnet_runtime::api::{
        self,
        runtime_types::{sp_core::ecdsa, tangle_primitives::services},
    },
    tx,
};
use std::net::IpAddr;

use incredible_squaring_blueprint::{self as blueprint, IncredibleSquaringTaskManager};

use alloy_signer::k256::PublicKey;
use gadget_common::config::DebugLogger;
use gadget_sdk::env::{AdditionalConfig, GadgetConfiguration};
use gadget_sdk::events_watcher::tangle::TangleConfig;
use gadget_sdk::keystore::BackendExt;
use gadget_sdk::network::gossip::GossipHandle;
use gadget_sdk::tangle_subxt::subxt;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::sp_core::ed25519;
use lock_api::{GuardSend, RwLock};
use sp_core::Pair;
use std::sync::Arc;

struct TangleGadgetRunner {
    env: StdGadgetConfiguration,
}

#[async_trait::async_trait]
impl GadgetRunner for TangleGadgetRunner {
    type Error = eyre::Report;

    fn env(&self) -> &StdGadgetConfiguration {
        &self.env
    }
    async fn register(&self) -> Result<()> {
        let env = self.env.clone();
        // TODO: Use the function in blueprint-test-utils
        if env.test_mode {
            env.logger.info("Skipping registration in test mode");
            return Ok(());
        }

        let client = env.client().await.map_err(|e| eyre!(e))?;
        let signer = env.first_signer().map_err(|e| eyre!(e))?;
        let ecdsa_pair = env.first_ecdsa_signer().map_err(|e| eyre!(e))?;

        let xt = api::tx().services().register(
            self.env.blueprint_id,
            services::OperatorPreferences {
                key: ecdsa::Public(ecdsa_pair.public_key().0),
                approval: services::ApprovalPrefrence::None,
            },
            Default::default(),
        );

        // send the tx to the tangle and exit.
        let result = tx::tangle::send(&client, &signer, &xt).await?;
        tracing::info!("Registered operator with hash: {:?}", result);
        Ok(())
    }

    async fn run(&self) -> Result<()> {
        let env = self.env.clone();
        let client = env.client().await.map_err(|e| eyre!(e))?;
        let signer = env.first_signer().map_err(|e| eyre!(e))?;

        let x_square = blueprint::XsquareEventHandler {
            service_id: env.service_id.unwrap(),
            signer,
        };

        tracing::info!("Starting the event watcher ...");

        SubstrateEventWatcher::run(
            &TangleEventsWatcher,
            client,
            // Add more handler here if we have more functions.
            vec![Box::new(x_square)],
        )
        .await?;

        Ok(())
    }

    async fn benchmark(&self) -> Result<()> {
        let env = gadget_sdk::env::load(None)?;
        let client = env.client().await?;
        let signer = env.first_signer()?;
        let xsquare_summary = blueprint::xsquare_benchmark();
        Ok(())
    }
}

fn create_gadget_runner(
    protocol: Protocol,
    additional_config: AdditionalConfig,
) -> Arc<dyn GadgetRunner> {
    let env = gadget_sdk::env::load(Some(protocol), additional_config)
        .expect("Failed to load environment");
    match protocol {
        Protocol::Tangle => Arc::new(TangleGadgetRunner { env }),
        Protocol::Eigenlayer => Arc::new(EigenlayerGadgetRunner { env }),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let env_filter = tracing_subscriber::EnvFilter::from_default_env();
    let logger = tracing_subscriber::fmt()
        .compact()
        .with_target(true)
        .with_env_filter(env_filter);
    logger.init();

    // TODO: Ideally, we don't want to have multiple types of loggers?
    let debug_logger = DebugLogger {
        id: "incredible-squaring".to_string(),
    };

    // Load the environment and create the gadget runner
    let protocol = Protocol::from_env().unwrap_or(Protocol::Tangle);
    let (env, runner) = create_gadget_runner(
        protocol,
        AdditionalConfig {
            bind_addr: IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
            bind_port: 0,
            test_mode: false,
            logger: debug_logger,
        },
    );
    // Register the operator if needed
    if runner.env().should_run_registration() {
        runner.register().await?;
    }
    // Run benchmark if needed
    if runner.env().should_run_benchmarks() {
        runner.benchmark().await?;
    }

    // Run the gadget / AVS
    runner.run().await?;

    Ok(())
}
