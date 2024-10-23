use crate::{
    aggregator::AggregatorContext,
    constants::{
        AVS_DIRECTORY_ADDRESS, DELEGATION_MANAGER_ADDRESS, EIGENLAYER_HTTP_ENDPOINT,
        OPERATOR_ADDRESS, OPERATOR_METADATA_URL, OPERATOR_STATE_RETRIEVER_ADDRESS, PRIVATE_KEY,
        REGISTRY_COORDINATOR_ADDRESS, SIGNATURE_EXPIRY, STRATEGY_MANAGER_ADDRESS,
        TASK_MANAGER_ADDRESS,
    },
    IncredibleSquaringTaskManager, InitializeBlsTaskEventHandler,
};
use alloy_network::EthereumWallet;
use alloy_primitives::{Bytes, FixedBytes, U256};
use alloy_provider::ProviderBuilder;
use alloy_signer_local::PrivateKeySigner;
use color_eyre::{eyre::eyre, Result};
use eigensdk::client_avsregistry::writer::AvsRegistryChainWriter;
use eigensdk::client_elcontracts::reader::ELChainReader;
use eigensdk::client_elcontracts::writer::ELChainWriter;
use eigensdk::crypto_bls::BlsKeyPair;
use eigensdk::logging::get_test_logger;
use eigensdk::types::operator::Operator;
use gadget_sdk::config::{ContextConfig, GadgetConfiguration};
use gadget_sdk::events_watcher::{evm::DefaultNodeConfig, InitializableEventHandler};
use gadget_sdk::info;
use gadget_sdk::run::GadgetRunner;
use gadget_sdk::structopt::StructOpt;
pub struct EigenlayerGadgetRunner<R: lock_api::RawRwLock> {
    pub env: GadgetConfiguration<R>,
}

impl<R: lock_api::RawRwLock> EigenlayerGadgetRunner<R> {
    pub async fn new(env: GadgetConfiguration<R>) -> Self {
        Self { env }
    }
}

#[async_trait::async_trait]
impl GadgetRunner for EigenlayerGadgetRunner<parking_lot::RawRwLock> {
    type Error = color_eyre::eyre::Report;

    fn config(&self) -> &GadgetConfiguration<parking_lot::RawRwLock> {
        todo!()
    }

    async fn register(&mut self) -> Result<()> {
        Ok(())
    }

    async fn benchmark(&self) -> std::result::Result<(), Self::Error> {
        todo!()
    }

    async fn run(&mut self) -> Result<()> {
        // Get the ECDSA key from the private key seed using alloy
        let signer: PrivateKeySigner = PRIVATE_KEY.parse().expect("failed to generate wallet ");
        let wallet = EthereumWallet::from(signer);

        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet.clone())
            .on_http(self.env.http_rpc_endpoint.clone().parse()?);

        let contract = IncredibleSquaringTaskManager::IncredibleSquaringTaskManagerInstance::new(
            *TASK_MANAGER_ADDRESS,
            provider.clone(),
        );

        let aggregator_context = AggregatorContext::new(
            "127.0.0.1:8081".to_string(),
            *TASK_MANAGER_ADDRESS,
            self.env.http_rpc_endpoint.clone(),
            wallet,
            self.env.clone(),
        )
        .await?;

        let x_square_eigen = InitializeBlsTaskEventHandler::<DefaultNodeConfig> {
            ctx: aggregator_context,
            contract: contract.into(),
        };

        info!("Contract address: {:?}", *TASK_MANAGER_ADDRESS);

        info!("Starting the Incredible Squaring event handler");
        let finished = x_square_eigen
            .init_event_handler()
            .await
            .expect("Event Listener init already called");
        info!("Event handler started...");
        let res = finished.await;
        info!("Event handler finished with {res:?}");
        Ok(())
    }
}

pub async fn execute_runner() -> Result<()> {
    gadget_sdk::logging::setup_log();
    let config = ContextConfig::from_args();
    let env = gadget_sdk::config::load(config).expect("Failed to load environment");
    let mut runner = Box::new(EigenlayerGadgetRunner::new(env.clone()).await);

    info!("~~~ Executing the incredible squaring blueprint ~~~");

    info!("Registering...");
    if env.should_run_registration() {
        runner.register().await?;
    }

    info!("Running...");
    runner.run().await?;

    info!("Exiting...");
    Ok(())
}
