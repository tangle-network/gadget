use std::path::{Path, PathBuf};
use std::time::Duration;
use testcontainers::{
    core::{ExecCommand, IntoContainerPort, WaitFor},
    runners::AsyncRunner,
    ContainerAsync, GenericImage, ImageExt,
};
use tokio::io::AsyncBufReadExt;
use tracing::info;

pub type Container = ContainerAsync<GenericImage>;

pub const ANVIL_IMAGE: &str = "ghcr.io/foundry-rs/foundry";
pub const ANVIL_TAG: &str = "v1.0.0";
pub const ANVIL_STATE_PATH: &str = "./anvil-testing-utils/data"; // relative path from the project root

const DEFAULT_ANVIL_STATE_PATH: &str = "./anvil-testing-utils/data/state.json"; // relative path from the project root

fn workspace_dir() -> PathBuf {
    let output = std::process::Command::new(env!("CARGO"))
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
            "-vvvv",
            "--host",
            "0.0.0.0",
            "--load-state",
            "/testnet_state.json",
            "--base-fee",
            "0",
            "--gas-price",
            "0",
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
