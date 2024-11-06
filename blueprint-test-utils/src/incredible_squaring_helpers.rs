use crate::anvil;
use alloy_primitives::{address, Address};
use std::sync::Arc;
use std::time::Duration;
use testcontainers::{ContainerAsync, GenericImage};
use tokio::sync::Mutex;

/// Starts an Anvil container for testing from the given state file in JSON format.
///
/// # Arguments
/// * `path` - The path to the save-state file.
/// * `include_logs` - If true, testnet output will be printed to the console.
///
/// # Returns
/// `(container, http_endpoint, ws_endpoint)`
///    - `container` as a [`ContainerAsync`] - The Anvil container.
///    - `http_endpoint` as a `String` - The Anvil HTTP endpoint.
///    - `ws_endpoint` as a `String` - The Anvil WS endpoint.
pub async fn start_anvil_testnet(
    path: &str,
    include_logs: bool,
) -> (ContainerAsync<GenericImage>, String, String) {
    let (container, http_endpoint, ws_endpoint) =
        anvil::start_anvil_container(path, include_logs).await;
    std::env::set_var("EIGENLAYER_HTTP_ENDPOINT", http_endpoint.clone());
    std::env::set_var("EIGENLAYER_WS_ENDPOINT", ws_endpoint.clone());

    // Sleep to give the testnet time to spin up
    tokio::time::sleep(Duration::from_secs(1)).await;
    (container, http_endpoint, ws_endpoint)
}

/// Waits for the given `successful_responses` Mutex to be greater than or equal to `task_response_count`.
pub async fn wait_for_responses(
    successful_responses: Arc<Mutex<usize>>,
    task_response_count: usize,
    timeout_duration: Duration,
) -> Result<Result<(), std::io::Error>, tokio::time::error::Elapsed> {
    tokio::time::timeout(timeout_duration, async move {
        loop {
            let count = *successful_responses.lock().await;
            if count >= task_response_count {
                crate::info!("Successfully received {} task responses", count);
                return Ok(());
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    })
    .await
}
