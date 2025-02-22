use crate::error::Error;
use alloy_contract::{CallBuilder, CallDecoder};
use alloy_provider::network::Ethereum;
use alloy_provider::Provider;
use alloy_rpc_types_eth::TransactionReceipt;
use alloy_transport::Transport;
use gadget_logging::{error, info};
use gadget_std::path::{Path, PathBuf};
use gadget_std::sync::{Arc, Mutex};
use gadget_std::time::Duration;
use testcontainers::{
    core::{ExecCommand, IntoContainerPort, WaitFor},
    runners::AsyncRunner,
    ContainerAsync, GenericImage, ImageExt,
};
use tokio::io::AsyncBufReadExt;

pub type Container = ContainerAsync<GenericImage>;

pub const ANVIL_IMAGE: &str = "ghcr.io/foundry-rs/foundry";
pub const ANVIL_TAG: &str = "nightly-5b7e4cb3c882b28f3c32ba580de27ce7381f415a";
pub const ANVIL_STATE_PATH: &str = "./crates/testing-utils/anvil/data"; // relative path from the project root

const DEFAULT_ANVIL_STATE_PATH: &str = "./crates/testing-utils/anvil/data/state.json"; // relative path from the project root

fn workspace_dir() -> PathBuf {
    let output = gadget_std::process::Command::new(env!("CARGO"))
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()
        .unwrap()
        .stdout;
    let cargo_path = Path::new(std::str::from_utf8(&output).unwrap().trim());
    cargo_path.parent().unwrap().to_path_buf()
}

/// Start an Anvil container for testing with contract state loaded.
pub async fn start_anvil_container(
    state_path: &str,
    include_logs: bool,
) -> (Container, String, String) {
    let relative_path = PathBuf::from(state_path);
    let absolute_path = workspace_dir().join(relative_path);
    let absolute_path_str = absolute_path.to_str().unwrap();

    if !absolute_path.exists() {
        panic!("Anvil state file not found at: {}", absolute_path.display());
    }

    let container = GenericImage::new(ANVIL_IMAGE, ANVIL_TAG)
        .with_wait_for(WaitFor::message_on_stdout("Listening on"))
        .with_exposed_port(8545.tcp())
        .with_entrypoint("anvil")
        .with_mount(testcontainers::core::Mount::bind_mount(
            absolute_path_str,
            "/testnet_state.json",
        ))
        .with_cmd([
            "--host",
            "0.0.0.0",
            "--load-state",
            "/testnet_state.json",
            "--base-fee",
            "0",
            "--gas-price",
            "0",
            "--code-size-limit",
            "50000",
            "--hardfork",
            "shanghai",
        ])
        .start()
        .await
        .expect("Error starting anvil container");

    if include_logs {
        let reader = container.stdout(true);
        tokio::task::spawn(async move {
            let mut reader = reader;
            let mut buffer = String::new();
            while reader.read_line(&mut buffer).await.unwrap() > 0 {
                println!("{:?}", buffer);
                buffer.clear();
            }
        });
    }

    mine_anvil_blocks(&container, 200).await;

    let port = container
        .ports()
        .await
        .unwrap()
        .map_to_host_port_ipv4(8545)
        .unwrap();

    let http_endpoint = format!("http://localhost:{}", port);
    println!("Anvil HTTP endpoint: {}", http_endpoint);
    let ws_endpoint = format!("ws://localhost:{}", port);
    println!("Anvil WS endpoint: {}", ws_endpoint);

    (container, http_endpoint, ws_endpoint)
}

/// Mine Anvil blocks.
pub async fn mine_anvil_blocks(container: &Container, n: u32) {
    let _output = container
        .exec(ExecCommand::new([
            "cast",
            "rpc",
            "anvil_mine",
            n.to_string().as_str(),
        ]))
        .await
        .expect("Failed to mine anvil blocks");
}

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
pub async fn start_anvil_testnet(path: &str, include_logs: bool) -> (Container, String, String) {
    let (container, http_endpoint, ws_endpoint) = start_anvil_container(path, include_logs).await;
    std::env::set_var("EIGENLAYER_HTTP_ENDPOINT", http_endpoint.clone());
    std::env::set_var("EIGENLAYER_WS_ENDPOINT", ws_endpoint.clone());

    // Sleep to give the testnet time to spin up
    tokio::time::sleep(Duration::from_secs(1)).await;
    (container, http_endpoint, ws_endpoint)
}

/// Starts an Anvil container for testing from this library's default state file.
///
/// # Arguments
/// * `include_logs` - If true, testnet output will be printed to the console.
///
/// # Returns
/// `(container, http_endpoint, ws_endpoint)`
///    - `container` as a [`ContainerAsync`] - The Anvil container.
///    - `http_endpoint` as a `String` - The Anvil HTTP endpoint.
///    - `ws_endpoint` as a `String` - The Anvil WS endpoint.
pub async fn start_default_anvil_testnet(include_logs: bool) -> (Container, String, String) {
    info!("Starting Anvil testnet from default state file");
    start_anvil_container(DEFAULT_ANVIL_STATE_PATH, include_logs).await
}

pub async fn get_receipt<T, P, D>(
    call: CallBuilder<T, P, D, Ethereum>,
) -> Result<TransactionReceipt, Error>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum>,
    D: CallDecoder,
{
    let pending_tx = match call.send().await {
        Ok(tx) => tx,
        Err(e) => {
            error!("Failed to send transaction: {:?}", e);
            return Err(e.into());
        }
    };

    let receipt = match pending_tx.get_receipt().await {
        Ok(receipt) => receipt,
        Err(e) => {
            error!("Failed to get transaction receipt: {:?}", e);
            return Err(e.into());
        }
    };

    Ok(receipt)
}

/// Waits for the given `successful_responses` Mutex to be greater than or equal to `task_response_count`.
pub async fn wait_for_responses(
    successful_responses: Arc<Mutex<usize>>,
    task_response_count: usize,
    timeout_duration: Duration,
) -> Result<Result<(), Error>, tokio::time::error::Elapsed> {
    tokio::time::timeout(timeout_duration, async move {
        loop {
            let count = match successful_responses.lock() {
                Ok(guard) => *guard,
                Err(e) => {
                    return Err(Error::WaitResponse(e.to_string()));
                }
            };
            if count >= task_response_count {
                info!("Successfully received {} task responses", count);
                return Ok(());
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    })
    .await
}
