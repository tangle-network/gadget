use crate::deploy::eigenlayer::{deploy_avs_contracts, EigenlayerDeployOpts};
use alloy_provider::RootProvider;
use alloy_transport::BoxTransport;
use color_eyre::eyre::Result;
use gadget_config::supported_chains::SupportedChains;
use gadget_logging::setup_log;
use gadget_std::collections::HashMap;
use gadget_std::fs;
use gadget_std::process::Command;
use gadget_testing_utils::anvil::start_default_anvil_testnet;
use gadget_utils::evm::get_provider_http;
use serde_json::Value;
use tempfile::TempDir;

#[tokio::test]
async fn test_deploy_local_on_anvil() -> Result<()> {
    setup_log();

    // Create a temporary directory for our test contract
    let temp_dir = TempDir::new()?;
    let contract_src_dir = temp_dir.path().join("src");
    let contract_out_dir = temp_dir.path().join("out");
    let contract_dir = temp_dir.path();
    fs::create_dir_all(&contract_src_dir)?;
    fs::create_dir_all(&contract_out_dir)?;

    let keystore_path = temp_dir.path().join("./keystore");

    // Write the test contract
    let contract_content = r#"// SPDX-License-Identifier: MIT
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
"#;

    fs::write(contract_src_dir.join("TestContract.sol"), contract_content)?;

    // Write the second test contract
    let second_contract_content = r#"// SPDX-License-Identifier: MIT
pragma solidity >=0.8.13;

contract SimpleStorage {
    string private storedData;
    event DataStored(string newData);

    constructor(string memory initialData) {
        storedData = initialData;
    }

    function set(string memory newData) public {
        storedData = newData;
        emit DataStored(newData);
    }

    function get() public view returns (string memory) {
        return storedData;
    }
}
"#;

    fs::write(
        contract_src_dir.join("SimpleStorage.sol"),
        second_contract_content,
    )?;

    // Create foundry.toml
    let foundry_content = format!(
        r#"[profile.default]
src = 'src'
out = '{}'
libs = ['lib']
evm_version = 'shanghai'"#,
        contract_out_dir
            .strip_prefix(temp_dir.path())
            .unwrap()
            .display()
    );
    fs::write(temp_dir.path().join("foundry.toml"), foundry_content)?;

    // Start the local Anvil testnet
    let (_container, http_endpoint, _ws_endpoint) = start_default_anvil_testnet(false).await;

    // Set up deployment options with temporary directory path and constructor arguments
    let mut constructor_args = HashMap::new();
    let init_a_value = 2;
    let init_b_value = 3;
    let init_get_value = init_a_value * init_b_value;
    constructor_args.insert(
        "TestContract".to_string(),
        vec![init_a_value.to_string(), init_b_value.to_string()],
    );
    constructor_args.insert(
        "SimpleStorage".to_string(),
        vec!["Initial Data".to_string()],
    );

    // Create the deployment options for the test
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
        .arg("--out")
        .arg(&contract_out_dir)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to build contracts");

    // Ensure all contracts were correctly built
    let test_contract_json = contract_out_dir.join("TestContract.sol/TestContract.json");
    let simple_storage_json = contract_out_dir.join("SimpleStorage.sol/SimpleStorage.json");
    assert!(test_contract_json.exists());
    assert!(simple_storage_json.exists());

    // Deploy the contract
    let contract_addresses = deploy_avs_contracts(&opts).await.unwrap();
    let &test_contract_address = contract_addresses
        .iter()
        .find(|(key, _value)| key.contains("TestContract"))
        .map(|(_key, value)| value)
        .expect("Could not find TestContract in deployed contracts");
    let &simple_storage_address = contract_addresses
        .iter()
        .find(|(key, _value)| key.contains("SimpleStorage"))
        .map(|(_key, value)| value)
        .expect("Could not find SimpleStorage in deployed contracts");

    // Read the ABI from the JSON file
    let json_path = temp_dir
        .path()
        .join("out/TestContract.sol/TestContract.json");
    let json_content = fs::read_to_string(json_path)?;
    let json: Value = serde_json::from_str(&json_content)?;
    let abi = json["abi"].to_string();
    let abi = alloy_json_abi::JsonAbi::from_json_str(&abi).unwrap();

    // Create a provider
    let provider = get_provider_http(&http_endpoint);

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

    // We will arbitrarily set the value to 123 for testing
    let value = alloy_dyn_abi::DynSolValue::from(alloy_primitives::U256::from(123));

    // Test the getValue function and ensure it returns the correct value
    let get_result = test_contract
        .function("getValue", &[])
        .unwrap()
        .call()
        .await
        .unwrap();
    let get_result_value: alloy_primitives::U256 =
        if let alloy_dyn_abi::DynSolValue::Uint(val, 256) = get_result[0] {
            val
        } else {
            panic!("Expected Uint256, but did not receive correct type")
        };
    assert_eq!(
        get_result_value,
        alloy_primitives::U256::from(init_get_value)
    );

    // Test the setValue function and ensure it returns successfully
    let set_result = test_contract
        .function("setValue", &[value])
        .unwrap()
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();
    assert!(set_result.status());

    // Test the getValue function and ensure it returns the newly set value
    let get_result = test_contract
        .function("getValue", &[])
        .unwrap()
        .call()
        .await
        .unwrap();
    let get_result_value: alloy_primitives::U256 =
        if let alloy_dyn_abi::DynSolValue::Uint(val, 256) = get_result[0] {
            val
        } else {
            panic!("Expected Uint256, but did not receive correct type")
        };
    assert_eq!(get_result_value, alloy_primitives::U256::from(123));

    // Read the ABI from the JSON file
    let json_path = temp_dir
        .path()
        .join("out/SimpleStorage.sol/SimpleStorage.json");
    let json_content = fs::read_to_string(json_path)?;
    let json: Value = serde_json::from_str(&json_content)?;
    let abi = json["abi"].to_string();
    let abi = alloy_json_abi::JsonAbi::from_json_str(&abi).unwrap();

    // Create a provider
    let provider = get_provider_http(&http_endpoint);

    // Create a contract instance
    let simple_storage_contract = alloy_contract::ContractInstance::<
        BoxTransport,
        RootProvider<BoxTransport>,
        alloy_network::Ethereum,
    >::new(
        simple_storage_address,
        provider.clone(),
        alloy_contract::Interface::new(abi),
    );

    // Verify the contract data
    let get_result = simple_storage_contract
        .function("get", &[])
        .unwrap()
        .call()
        .await
        .unwrap();
    let get_result_value: String =
        if let alloy_dyn_abi::DynSolValue::String(val) = get_result[0].clone() {
            val
        } else {
            panic!("Expected String, but did not receive correct type")
        };
    assert_eq!(get_result_value, "Initial Data".to_string());

    Ok(())
}
