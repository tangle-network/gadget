use blueprint_test_utils::anvil;
// use blueprint_test_utils::eigenlayer_test_env::{
//     setup_eigenlayer_test_environment, EigenlayerTestEnvironment,
// };
use blueprint_test_utils::helpers::BlueprintProcessManager;
// use blueprint_test_utils::incredible_squaring_helpers::{
//     deploy_task_manager, setup_task_response_listener, setup_task_spawner, wait_for_responses,
// };
// use gadget_sdk::config::protocol::EigenlayerContractAddresses;
use gadget_sdk::config::Protocol;
use gadget_sdk::logging::setup_log;
// use gadget_sdk::{error, info};
use std::path::PathBuf;
// use std::sync::Arc;
// use std::time::Duration;
// use tokio::sync::Mutex;

blueprint_test_utils::define_eigenlayer_test_env!();

const ANVIL_STATE_PATH: &str =
    "./../blueprint-test-utils/anvil/deployed_anvil_states/testnet_state.json";

#[tokio::test(flavor = "multi_thread")]
#[allow(clippy::needless_return)]
async fn test_eigenlayer_incredible_squaring_blueprint() {
    setup_log();

    let (_container, http_endpoint, ws_endpoint) =
        anvil::start_anvil_container(ANVIL_STATE_PATH, true).await;

    std::env::set_var("EIGENLAYER_HTTP_ENDPOINT", http_endpoint.clone());
    std::env::set_var("EIGENLAYER_WS_ENDPOINT", ws_endpoint.clone());
    // Sleep to give the testnet time to spin up
    tokio::time::sleep(Duration::from_secs(1)).await;

    let EigenlayerTestEnvironment {
        accounts,
        http_endpoint,
        ws_endpoint,
        eigenlayer_contract_addresses:
            EigenlayerContractAddresses {
                registry_coordinator_address,
                ..
            },
        pauser_registry_address,
        ..
    } = setup_eigenlayer_test_environment(&http_endpoint, &ws_endpoint).await;
    let owner_address = &accounts[1];
    let aggregator_address = &accounts[9];
    let task_generator_address = &accounts[4];
    let task_manager_address = deploy_task_manager(
        &http_endpoint,
        registry_coordinator_address,
        pauser_registry_address,
        *owner_address,
        *aggregator_address,
        *task_generator_address,
    )
    .await;

    let num_successful_responses_required = 3;
    let successful_responses = Arc::new(Mutex::new(0));
    let successful_responses_clone = successful_responses.clone();

    // Start the Task Response Listener
    let response_listener = setup_task_response_listener(
        task_manager_address,
        ws_endpoint.clone(),
        successful_responses,
    )
    .await;

    // Start the Task Spawner
    let task_spawner = setup_task_spawner(
        task_manager_address,
        registry_coordinator_address,
        *task_generator_address,
        accounts.to_vec(),
        http_endpoint.clone(),
    )
    .await;

    tokio::spawn(async move {
        task_spawner.await;
    });

    tokio::spawn(async move {
        response_listener.await;
    });

    info!("Starting Blueprint Binary...");

    let blueprint_process_manager = BlueprintProcessManager::new();
    let current_dir = std::env::current_dir().unwrap();
    let xsquare_task_program_path = PathBuf::from(format!(
        "{}/../target/release/incredible-squaring-blueprint-eigenlayer",
        current_dir.display()
    ))
    .canonicalize()
    .unwrap();

    let tmp_dir = tempfile::TempDir::new().unwrap();
    let keystore_path = &format!("{}", tmp_dir.path().display());

    blueprint_process_manager
        .start_blueprints(
            vec![xsquare_task_program_path],
            &http_endpoint,
            ws_endpoint.as_ref(),
            Protocol::Eigenlayer,
            keystore_path,
        )
        .await
        .unwrap();

    // Wait for the process to complete or timeout
    let timeout_duration = Duration::from_secs(300);
    let result = wait_for_responses(
        successful_responses_clone.clone(),
        num_successful_responses_required,
        timeout_duration,
    )
    .await;

    // Check the result
    if let Ok(Ok(())) = result {
        info!("Test completed successfully with {num_successful_responses_required} tasks responded to.");
        blueprint_process_manager
            .kill_all()
            .await
            .unwrap_or_else(|e| {
                error!("Failed to kill all blueprint processes: {:?}", e);
            });
    } else {
        panic!(
            "Test timed out after {} seconds with {} successful responses out of {} required",
            timeout_duration.as_secs(),
            successful_responses_clone.lock().await,
            num_successful_responses_required
        );
    }
}
