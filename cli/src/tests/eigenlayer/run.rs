use blueprint_runner::config::BlueprintEnvironment;
use blueprint_runner::config::SupportedChains;
use blueprint_runner::config::{ContextConfig, Protocol, ProtocolSettings};
use blueprint_runner::eigenlayer::config::EigenlayerProtocolSettings;
use color_eyre::eyre::Result;
use gadget_chain_setup::anvil::start_default_anvil_testnet;
use gadget_std::collections::HashMap;
use gadget_std::fs;
use gadget_std::process::Command;
use gadget_testing_utils::setup_log;
use tempfile::TempDir;

use crate::command::deploy::eigenlayer::EigenlayerDeployOpts;
use crate::command::deploy::eigenlayer::deploy_avs_contracts;
use crate::command::run::run_eigenlayer_avs;

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_run_eigenlayer_avs() -> Result<()> {
    setup_log();

    // Create a temporary directory for our test contract and binary
    let temp_dir = TempDir::new()?;
    let contract_src_dir = temp_dir.path().join("src");
    let contract_out_dir = temp_dir.path().join("out");
    let contract_dir = temp_dir.path();
    fs::create_dir_all(&contract_src_dir)?;
    fs::create_dir_all(&contract_out_dir)?;

    let keystore_path = temp_dir.path().join("./keystore");

    // Write the test contract
    let contract_content = r"// SPDX-License-Identifier: MIT
pragma solidity >=0.8.13;

contract TestContract {
    uint256 private value;
    event ValueSet(uint256 newValue);

    constructor(uint256 a, uint256 b) {
        value = a * b;
    }

    function setValue(uint256 _value) public {
        value = _value;
        emit ValueSet(_value);
    }

    function getValue() public view returns (uint256) {
        return value;
    }
}
";
    fs::write(contract_src_dir.join("TestContract.sol"), contract_content)?;

    // Create foundry.toml
    let foundry_content = format!(
        r"[profile.default]
src = 'src'
out = '{}'
libs = ['lib']
evm_version = 'shanghai'",
        contract_out_dir
            .strip_prefix(temp_dir.path())
            .unwrap()
            .display()
    );
    fs::write(temp_dir.path().join("foundry.toml"), foundry_content)?;

    // Start the local Anvil testnet
    let (_container, http_endpoint, ws_endpoint) = start_default_anvil_testnet(false).await;

    // Set up deployment options with temporary directory path and constructor arguments
    let mut constructor_args = HashMap::new();
    let init_a_value = 8;
    let init_b_value = 11;
    let expected_value = init_a_value * init_b_value;
    constructor_args.insert(
        "TestContract".to_string(),
        vec![init_a_value.to_string(), init_b_value.to_string()],
    );

    // Deploy the contract
    let opts = EigenlayerDeployOpts {
        rpc_url: http_endpoint.clone(),
        contracts_path: contract_dir.to_string_lossy().to_string(),
        constructor_args: Some(constructor_args),
        ordered_deployment: false,
        chain: SupportedChains::LocalTestnet,
        keystore_path: keystore_path.to_string_lossy().to_string(),
    };

    // Build the contracts in temporary directory
    let _build_output = Command::new("forge")
        .arg("build")
        .arg("--evm-version")
        .arg("shanghai")
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to build contracts");

    // Deploy contracts
    let contract_addresses = deploy_avs_contracts(&opts)?;
    let test_contract_address = contract_addresses
        .iter()
        .find(|(key, _value)| key.contains("TestContract"))
        .map(|(_key, value)| value)
        .expect("Could not find TestContract in deployed contracts");

    // Create a binary that will interact with the contract
    let binary_dir = temp_dir.path().join("binary");
    fs::create_dir_all(&binary_dir)?;

    // Create Cargo.toml for the binary
    #[allow(clippy::useless_format)]
    let cargo_toml = format!(
        r#"[package]
name = "testing"
version = "0.1.0"
edition = "2021"

[dependencies]
blueprint-sdk = {{ git = "https://github.com/tangle-network/gadget.git", default-features = false, features = ["std", "eigenlayer", "evm", "macros", "build"] }}
tokio = {{ version = "1.40", features = ["full"] }}
color-eyre = "0.6"
alloy-primitives = {{ version = "0.8" }}
alloy-sol-types = {{ version = "0.8" }}
alloy-transport = {{ version = "0.9" }}
alloy-transport-http = {{ version = "0.9" }}
alloy-json-rpc = {{ version = "0.9" }}
alloy-provider = {{ version = "0.9", features = ["reqwest", "ws"] }}
alloy-rpc-client = {{ version = "0.9" }}
alloy-json-abi = {{ version = "0.8" }}
alloy-dyn-abi = {{ version = "0.8" }}
alloy-contract = {{ version = "0.9" }}
alloy-network = {{ version = "0.9" }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
"#
    );
    fs::write(binary_dir.join("Cargo.toml"), cargo_toml)?;

    // Create src directory for the binary
    fs::create_dir_all(binary_dir.join("src"))?;

    // Create a success file path
    let success_file = temp_dir.path().join("run_succeeded");
    let success_file_str = success_file.to_string_lossy();

    // Create the binary that will interact with the contract
    let main_rs = format!(
        r#"use blueprint_sdk::alloy::primitives::Address;
use blueprint_sdk::info;
use blueprint_sdk::std::{{string::ToString, fs, path::PathBuf}};
use alloy_sol_types::sol;
use alloy_transport::BoxTransport;
use alloy_provider::RootProvider;
use serde_json::Value;
use blueprint_sdk::utils::evm::get_provider_http;

#[blueprint_sdk::main(env)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    info!("~~~ Test AVS Started ~~~");

    let test_contract_address: Address = "{}".parse().expect("Invalid TEST_CONTRACT_ADDRESS");
    info!("Test contract address: {{}}", test_contract_address);

    let temp_dir_str: String = "{}".to_string();
    info!("Temp dir str: {{}}", temp_dir_str);

    // Create a provider
    let http_url = env.http_rpc_endpoint.clone();
    let provider = get_provider_http(&http_url);

    // Read the ABI from the JSON file
    let json_path = PathBuf::from(&temp_dir_str).join("out/TestContract.sol/TestContract.json");
    let json_content = fs::read_to_string(json_path)?;
    let json: Value = serde_json::from_str(&json_content)?;
    let abi = json["abi"].to_string();
    let abi = alloy_json_abi::JsonAbi::from_json_str(&abi).unwrap();
    info!("Successfully read ABI");

    // Create a contract instance
    let test_contract = alloy_contract::ContractInstance::<
        BoxTransport,
        RootProvider<BoxTransport>,
        alloy_network::Ethereum,
    >::new(
        test_contract_address,
        provider.clone(),
        alloy_contract::Interface::new(abi),
    );
    info!("Successfully created contract instance");

    // Test the getValue function
    let get_result = test_contract
        .function("getValue", &[])
        .unwrap()
        .call()
        .await
        .unwrap();
    info!("Successfully called getValue function");

    let get_result_value: alloy_primitives::U256 =
        if let alloy_dyn_abi::DynSolValue::Uint(val, 256) = get_result[0] {{
            val
        }} else {{
            panic!("Expected Uint256, but did not receive correct type")
        }};

    info!("Contract returned value: {{}}", get_result_value);

    if get_result_value == alloy_primitives::U256::from({}) {{
        info!("Writing success file");
        fs::write("{}", "")?;
    }}

    Ok(())
}}
"#,
        test_contract_address,
        temp_dir.path().display(),
        expected_value,
        success_file_str
    );
    fs::write(binary_dir.join("src/main.rs"), main_rs)?;

    // Build the binary
    let build_output = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .current_dir(&binary_dir)
        .output()
        .expect("Failed to build binary");
    if !build_output.status.success() {
        blueprint_core::debug!("Cargo build output: {:?}", build_output);
        panic!("Failed to build binary")
    }

    let binary_path = binary_dir.join("target/release");
    assert!(
        binary_path.exists(),
        "Binary path not found at: {}",
        binary_path.display()
    );

    let binary_path = binary_path.join("testing");

    // Run the binary using the run command
    let config = ContextConfig::create_config(
        http_endpoint.parse().unwrap(),
        ws_endpoint.parse().unwrap(),
        keystore_path.to_string_lossy().to_string(),
        None,
        SupportedChains::LocalTestnet,
        Protocol::Eigenlayer,
        ProtocolSettings::Eigenlayer(EigenlayerProtocolSettings::default()),
    );

    let run_opts = BlueprintEnvironment::load_with_config(config)
        .expect("Failed to load BlueprintEnvironment");

    // Run the AVS
    let mut child =
        run_eigenlayer_avs(run_opts, SupportedChains::LocalTestnet, Some(binary_path)).await?;

    // Update the success detection loop
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(2000));
    let mut attempts = 0;
    const MAX_ATTEMPTS: u32 = 30; // 60 seconds total timeout

    loop {
        blueprint_core::info!(
            "Waiting for run to succeed (attempt {}/{})",
            attempts + 1,
            MAX_ATTEMPTS
        );

        if success_file.exists() {
            blueprint_core::info!("Run succeeded!");
            break;
        }

        attempts += 1;
        assert!(
            attempts < MAX_ATTEMPTS,
            "Test timed out waiting for success file"
        );

        interval.tick().await;
    }

    child.wait().await.unwrap();

    Ok(())
}
