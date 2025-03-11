use alloy_contract::{CallBuilder, CallDecoder};
use alloy_primitives::{Uint, address};
use alloy_provider::Provider;
use alloy_provider::network::Ethereum;
use alloy_rpc_types_eth::TransactionReceipt;
use dialoguer::console::style;
use eigensdk::utils::rewardsv2::middleware::ecdsastakeregistry::ECDSAStakeRegistry;
use eigensdk::utils::rewardsv2::middleware::ecdsastakeregistry::ECDSAStakeRegistry::Quorum;
use eigensdk::utils::slashing::middleware::registrycoordinator::ISlashingRegistryCoordinatorTypes::OperatorSetParam;
use eigensdk::utils::slashing::middleware::registrycoordinator::IStakeRegistryTypes::StrategyParams;
use eigensdk::utils::slashing::middleware::registrycoordinator::RegistryCoordinator;
use gadget_logging::{error, info};
use std::fs::{self};
use tempfile::TempDir;
use testcontainers::{
    ContainerAsync, GenericImage, ImageExt,
    core::{ExecCommand, IntoContainerPort, WaitFor},
    runners::AsyncRunner,
};
use tokio::io::AsyncBufReadExt;

mod error;
mod state;

use blueprint_evm_extra::util::get_provider_http;
pub use error::Error;
pub use state::{AnvilState, get_default_state, get_default_state_json};

pub type Container = ContainerAsync<GenericImage>;

pub const ANVIL_IMAGE: &str = "ghcr.io/foundry-rs/foundry";
pub const ANVIL_TAG: &str = "nightly-5b7e4cb3c882b28f3c32ba580de27ce7381f415a";

/// Print a section header with consistent styling
pub fn print_section_header(title: &str) {
    println!("\n{}", style(format!("━━━ {} ━━━", title)).cyan().bold());
}

/// Print a success message with an emoji and optional details
pub fn print_success(message: &str, details: Option<&str>) {
    println!(
        "\n{}  {}{}",
        style("✓").green().bold(),
        style(message).green(),
        details.map_or(String::new(), |d| format!("\n   {}", style(d).dim()))
    );
}

/// Print an info message with consistent styling
pub fn print_info(message: &str) {
    println!("{}", style(format!("ℹ {}", message)).blue());
}

/// Write settings to .env file and display them
fn write_and_display_settings(
    http_endpoint: &str,
    ws_endpoint: &str,
    ecdsa_registry: &str,
) -> std::io::Result<()> {
    // Display current deployment variables
    print_section_header("Deployment Variables");
    let deployment_vars = format!(
        "# Network Settings\nANVIL_HTTP_ENDPOINT={}\nANVIL_WS_ENDPOINT={}\n\n# Contract Registry\nECDSA_STAKE_REGISTRY_ADDRESS={}\n",
        http_endpoint, ws_endpoint, ecdsa_registry
    );
    println!("{}", style(deployment_vars.trim()).dim());

    // Check and display existing settings if they exist
    let settings_path = "settings.env";
    if std::path::Path::new(settings_path).exists() {
        print_section_header("Current Environment");
        let existing_content = fs::read_to_string(settings_path)?;
        println!(
            "{}",
            style(format!("📄 {}:", settings_path)).yellow().bold()
        );
        println!("{}", style(existing_content.trim()).dim());
    }

    Ok(())
}

/// Start an Anvil container for testing with contract state loaded.
#[allow(clippy::missing_panics_doc)] // TODO(serial): Return an error, no panics
#[allow(clippy::too_many_lines)]
pub async fn start_anvil_container(
    state_json: &str,
    include_logs: bool,
) -> (Container, String, String, Option<TempDir>) {
    print_section_header("Starting Anvil Container");
    print_info("Initializing temporary environment...");

    // Create a temporary directory and write the state file
    let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
    let state_path = temp_dir.path().join("state.json");
    fs::write(&state_path, state_json).expect("Failed to write state file");

    print_info("Starting Anvil node...");
    let container = match GenericImage::new(ANVIL_IMAGE, ANVIL_TAG)
        .with_wait_for(WaitFor::message_on_stdout("Listening on"))
        .with_exposed_port(8545.tcp())
        .with_entrypoint("anvil")
        .with_mount(testcontainers::core::Mount::bind_mount(
            state_path.to_str().unwrap(),
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
    {
        Ok(container) => container,
        Err(e) => {
            let error_msg = e.to_string().to_lowercase();
            if error_msg.contains("connection refused") {
                println!(
                    "\n{}",
                    style("Error: Docker Connection Failed").red().bold()
                );
                println!(
                    "{}",
                    style("Could not connect to Docker. Please ensure:").red()
                );
                println!("1. Docker Desktop is installed on your machine");
                println!("   Download from: https://www.docker.com/products/docker-desktop");
                println!("2. Docker Desktop is running");
                println!("3. Docker daemon is accessible");
                std::process::exit(1);
            } else if error_msg.contains("not found") {
                println!("\n{}", style("Error: Docker Image Not Found").red().bold());
                println!(
                    "{}",
                    style("The required Docker image could not be found. Please ensure:").red()
                );
                println!("1. You have an active internet connection");
                println!("2. You have sufficient permissions to pull Docker images");
                std::process::exit(1);
            }
            #[allow(clippy::unnecessary_literal_unwrap)]
            return Err(color_eyre::eyre::eyre!(
                "Failed to start Anvil container: {}",
                e
            ))
            .expect("Container start error");
        }
    };

    if include_logs {
        let reader = container.stdout(true);
        tokio::task::spawn(async move {
            let mut reader = reader;
            let mut buffer = String::new();
            while reader.read_line(&mut buffer).await.unwrap() > 0 {
                info!("{:?}", buffer);
                buffer.clear();
            }
        });
    }

    print_info("Mining initial blocks...");
    mine_anvil_blocks(&container, 200).await;

    let port = container
        .ports()
        .await
        .unwrap()
        .map_to_host_port_ipv4(8545)
        .unwrap();

    let http_endpoint = format!("http://localhost:{}", port);
    let ws_endpoint = format!("ws://localhost:{}", port);

    print_section_header("Network Endpoints");
    println!(
        "{}: {}",
        style("HTTP").cyan(),
        style(&http_endpoint).yellow()
    );
    println!(
        "{}: {}",
        style("WebSocket").cyan(),
        style(&ws_endpoint).yellow()
    );

    // Display funded test accounts
    print_section_header("Funded Test Accounts");
    println!("{}", style("Available accounts").cyan());
    println!("Each account is funded with 10000 ETH\n");

    let test_accounts = [
        (
            "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
        ),
        (
            "0x70997970C51812dc3A010C7d01b50e0d17dc79C8",
            "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d",
        ),
        (
            "0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC",
            "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a",
        ),
        (
            "0x90F79bf6EB2c4f870365E785982E1f101E93b906",
            "0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6",
        ),
        (
            "0x15d34AAf54267DB7D7c367839AAf71A00a2C6A65",
            "0x47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733639f19c30a34926a",
        ),
        (
            "0x9965507D1a55bcC2695C58ba16FB37d819B0A4dc",
            "0x8b3a350cf5c34c9194ca85829a2df0ec3153be0318b5e2d3348e872092edffba",
        ),
        (
            "0x976EA74026E726554dB657fA54763abd0C3a0aa9",
            "0x92db14e403b83dfe3df233f83dfa3a0d7096f21ca9b0d6d6b8d88b2b4ec1564e",
        ),
        (
            "0x14dC79964da2C08b23698B3D3cc7Ca32193d9955",
            "0x4bbbf85ce3377467afe5d46f804f221813b2bb87f24d81f60f1fcdbf7cbf4356",
        ),
        (
            "0x23618e81E3f5cdF7f54C3d65f7FBc0aBf5B21E8f",
            "0xdbda1821b80551c9d65939329250298aa3472ba22feea921c0cf5d620ea67b97",
        ),
        (
            "0xa0Ee7A142d267C1f36714E4a8F75612F20a79720",
            "0x2a871d0798f97d79848a013d4936a73bf4cc922c825d33c1cf7073dff6d409c6",
        ),
    ];

    for (i, (address, private_key)) in test_accounts.iter().enumerate() {
        println!(
            "({}) Address: {}\n    Private Key: {}",
            style(i).cyan(),
            style(address).yellow(),
            style(private_key).dim()
        );
    }

    let provider = get_provider_http(&http_endpoint);

    print_section_header("Contract Deployment");
    print_info("Deploying core contracts...");

    // Some setup is required before we can interact with the contract
    // let accounts = provider.get_accounts().await.unwrap();
    // let pauser_registry = PauserRegistry::deploy(provider.clone(), accounts.clone(), accounts[0])
    //     .await
    //     .unwrap();
    // let pauser_registry_address = *pauser_registry.address();
    // print_success(
    //     "Pauser Registry deployed",
    //     Some(&format!("Address: {}", pauser_registry_address)),
    // );

    let delegation_manager_address = address!("dc64a140aa3e981100a9beca4e685f962f0cf6c9");
    let service_manager_address = address!("34B40BA116d5Dec75548a9e9A8f15411461E8c70");

    let ecdsa_stake_registry =
        ECDSAStakeRegistry::deploy(provider.clone(), delegation_manager_address)
            .await
            .unwrap();
    let ecdsa_stake_registry_address = *ecdsa_stake_registry.address();
    print_success(
        "ECDSA Stake Registry deployed",
        Some(&format!("Address: {}", ecdsa_stake_registry_address)),
    );

    let registry_coordinator_address =
        alloy_primitives::address!("c3e53f4d16ae77db1c982e75a937b9f60fe63690");
    let registry_coordinator =
        RegistryCoordinator::new(registry_coordinator_address, provider.clone());

    print_info("\nConfiguring registry parameters...");
    let operator_set_params = OperatorSetParam {
        maxOperatorCount: 10,
        kickBIPsOfOperatorStake: 100,
        kickBIPsOfTotalStake: 1000,
    };
    let erc20_mock_address = address!("7969c5ed335650692bc04293b07f5bf2e7a673c0");
    let strategy_params = StrategyParams {
        strategy: erc20_mock_address,
        multiplier: Uint::from(10000),
    };
    let strategies = vec![strategy_params];

    print_info("Creating Quorum...");
    let _receipt = get_receipt(registry_coordinator.createTotalDelegatedStakeQuorum(
        operator_set_params,
        Uint::from(0),
        strategies,
    ))
    .await
    .unwrap();

    let threshold_weight = alloy_primitives::U256::from(1);
    let strategy_params = ECDSAStakeRegistry::StrategyParams {
        strategy: erc20_mock_address,
        multiplier: Uint::from(10000),
    };
    let strategies = vec![strategy_params];
    let quorum = Quorum { strategies };

    print_info("Initializing ECDSA Stake Registry...");
    let ecdsa_stake_registry_init = ecdsa_stake_registry
        .initialize(service_manager_address, threshold_weight, quorum)
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();

    if !ecdsa_stake_registry_init.status() {
        print_section_header("Deployment Failed");
        println!(
            "{}",
            style("Failed to initialize ECDSA Stake Registry")
                .red()
                .bold()
        );
        panic!("Failed to initialize ECDSA Stake Registry");
    }

    print_success("ECDSA Stake Registry initialized", None);

    // Write and display settings
    if let Err(e) = write_and_display_settings(
        &http_endpoint,
        &ws_endpoint,
        &ecdsa_stake_registry_address.to_string(),
    ) {
        error!("Failed to write settings: {}", e);
    }

    print_section_header("Setup Complete");
    println!(
        "{}",
        style("Your local Anvil environment is ready!")
            .green()
            .bold()
    );

    (container, http_endpoint, ws_endpoint, Some(temp_dir))
}

/// Mine Anvil blocks.
#[allow(clippy::missing_panics_doc)] // TODO(serial): Return errors, not panics
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

/// Starts an Anvil container for testing with the default state.
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
    let (container, http, ws, _) =
        start_anvil_container(get_default_state_json(), include_logs).await;
    (container, http, ws)
}

/// Starts an Anvil container for testing with custom state.
///
/// # Arguments
/// * `state` - The state to load into Anvil.
/// * `include_logs` - If true, testnet output will be printed to the console.
///
/// # Returns
/// `(container, http_endpoint, ws_endpoint)`
///    - `container` as a [`ContainerAsync`] - The Anvil container.
///    - `http_endpoint` as a `String` - The Anvil HTTP endpoint.
///    - `ws_endpoint` as a `String` - The Anvil WS endpoint.
#[allow(clippy::missing_panics_doc)] // TODO(serial): Return errors, not panics
pub async fn start_anvil_testnet_with_state(
    state: &AnvilState,
    include_logs: bool,
) -> (Container, String, String) {
    let state_json = serde_json::to_string(state).expect("Failed to serialize state");
    let (container, http, ws, _) = start_anvil_container(&state_json, include_logs).await;
    (container, http, ws)
}

async fn get_receipt<T, P, D>(
    call: CallBuilder<T, P, D, Ethereum>,
) -> Result<TransactionReceipt, Error>
where
    P: Provider<Ethereum>,
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
