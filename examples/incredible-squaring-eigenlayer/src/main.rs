//! Incredible Squaring TaskManager Monitor
//!
//! Monitors TaskManager events for task creation and completion.

use ::std::{str::FromStr, sync::Arc, time::Duration};
use alloy_primitives::Address;
use blueprint_evm_extra::producer::{PollingConfig, PollingProducer};
use blueprint_runner::eigenlayer::bls::EigenlayerBLSConfig;
use blueprint_runner::{config::BlueprintEnvironment, BlueprintRunner};
use blueprint_sdk::utils::evm::get_provider_http;
use blueprint_sdk::*;
use gadget_logging::setup_log;
use incredible_squaring_eigenlayer::{create_contract_router, ExampleContext};
use tracing_subscriber::filter::LevelFilter;

#[tokio::main]
async fn main() -> Result<(), blueprint_sdk::Error> {
    setup_log();

    // Get contract address from environment
    let task_manager = std::env::var("TASK_MANAGER_ADDRESS")
        .map(|addr| Address::from_str(&addr))
        .expect("TASK_MANAGER_ADDRESS must be set")?;

    // Create RPC client
    let rpc_url = std::env::var("RPC_URL").expect("RPC_URL must be set");
    let client = get_provider_http(&rpc_url);

    let client = Arc::new(client);
    // Create producer for task events
    let task_producer = PollingProducer::new(
        client.clone(),
        PollingConfig {
            poll_interval: Duration::from_secs(1),
            ..Default::default()
        },
    );

    // Create and run the blueprint
    let eigenlayer_bls_config = EigenlayerBLSConfig::new(
        Address::from_str("0x0000000000000000000000000000000000000000").unwrap(),
        Address::from_str("0x0000000000000000000000000000000000000000").unwrap(),
    );
    let ctx = ExampleContext {};
    BlueprintRunner::builder(eigenlayer_bls_config, BlueprintEnvironment::default())
        .router(create_contract_router(ctx, task_manager))
        .producer(task_producer)
        .with_shutdown_handler(async {
            tracing::info!("Shutting down task manager service");
        })
        .run()
        .await?;

    Ok(())
}
