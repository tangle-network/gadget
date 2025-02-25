//! Incredible Squaring TaskManager Monitor
//!
//! Monitors TaskManager events for task creation and completion.

use crate::contracts::SquaringTask::NewTaskCreated;
use alloy_sol_types::SolEvent;
use blueprint_core::*;
use blueprint_evm_extra::extract::{BlockNumber, Events};
use blueprint_evm_extra::filters::{contract::MatchesContract, event::MatchesEvent};
use blueprint_router::Router;
use tower::filter::FilterLayer;

pub mod config;
pub mod contracts;

/// Service context shared between jobs
#[derive(Debug, Clone)]
pub struct ExampleContext {}

/// Job function for handling tasks
pub async fn handle_task(
    BlockNumber(block_number): BlockNumber,
    Context(_): Context<ExampleContext>,
    Events(tasks): Events<NewTaskCreated>,
) -> impl IntoJobResult {
    for task in tasks {
        tracing::info!(
            "Processing task: index={}, generator={}, quorum={}, block_number={}",
            task.taskIndex,
            task.task.message,
            task.task.quorumNumbers,
            block_number
        );

        // Here you would implement the actual task handling logic
        // For example:
        // - Validate task parameters
        // - Process task data
        // - Submit response
    }
    JobResult::new(Vec::new())
}

/// Creates a router with task event filters
pub fn create_contract_router(contract_address: alloy_primitives::Address) -> Router {
    Router::new()
        .route(*NewTaskCreated::SIGNATURE_HASH, handle_task)
        .with_context(ExampleContext {})
        .layer(FilterLayer::new(MatchesEvent(
            NewTaskCreated::SIGNATURE_HASH,
        )))
        .layer(FilterLayer::new(MatchesContract(contract_address)))
}

#[cfg(test)]
mod tests {
    use crate::{config::get_provider_http, contracts::SquaringTask};

    use super::*;
    use alloy_primitives::{address, Address, U256};
    use alloy_provider::Provider;
    use blueprint_evm_extra::producer::{PollingConfig, PollingProducer};
    use blueprint_runner::{
        config::GadgetConfiguration, error::RunnerError, BlueprintConfig, BlueprintRunner,
    };
    use gadget_anvil_testing_utils::start_default_anvil_testnet;
    use gadget_logging::setup_log;
    use std::{sync::Arc, time::Duration};

    const REGISTRY_COORDINATOR_ADDRESS: Address =
        address!("0xc3e53f4d16ae77db1c982e75a937b9f60fe63690");

    /// Address of the first account in the local anvil network
    const ANVIL_FIRST_ADDRESS: Address = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");

    #[derive(Debug, Clone, Copy)]
    struct MockBlueprintConfig;

    #[async_trait::async_trait]
    impl BlueprintConfig for MockBlueprintConfig {
        async fn requires_registration(
            &self,
            _env: &GadgetConfiguration,
        ) -> Result<bool, RunnerError> {
            Ok(false)
        }
    }

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
                    let block_number = tx.provider().get_block_number().await?;
                    tracing::info!(
                        "Submitted squaring task for number {} in tx {:?} (block: {})",
                        number,
                        tx.tx_hash(),
                        block_number,
                    );
                    number += 1;
                }
                Err(e) => {
                    tracing::error!("Failed to submit task: {}", e);
                }
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn task_monitoring() -> Result<(), BoxError> {
        setup_log();
        let (_anvil_container, rpc_url, _ws_url) = start_default_anvil_testnet(false).await;

        let provider = get_provider_http(&rpc_url);
        // Deploy contracts and get addresses
        let generator = ANVIL_FIRST_ADDRESS;
        let aggregator = ANVIL_FIRST_ADDRESS;
        let owner = ANVIL_FIRST_ADDRESS;

        let register_coordinatior = REGISTRY_COORDINATOR_ADDRESS;
        let task_response_window = 1000;
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
                poll_interval: Duration::from_secs(1),
                start_block: 205,
                confirmations: 0,
                ..Default::default()
            },
        );

        // Submit task
        let handle = tokio::spawn(submit_tasks(provider.clone(), *contract.address()));

        // Create and run the blueprint
        BlueprintRunner::new(MockBlueprintConfig, Default::default())
            .router(create_contract_router(*contract.address()))
            .producer(task_polling_producer)
            .run()
            .await?;

        handle.abort();

        Ok(())
    }
}
