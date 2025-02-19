use crate::deploy::eigenlayer::{deploy_avs_contracts, EigenlayerDeployOpts};
use crate::keys::generate_key;
use crate::signer::{load_evm_signer_from_env, load_signer_from_env, EVM_SIGNER_ENV, SIGNER_ENV};
use alloy_provider::RootProvider;
use alloy_transport::BoxTransport;
use color_eyre::eyre::Result;
use gadget_crypto::bn254::ArkBlsBn254;
use gadget_crypto::sp_core::{SpBls381, SpEcdsa, SpEd25519, SpSr25519};
use gadget_crypto_core::KeyTypeId;
use gadget_keystore::backends::Backend;
use gadget_keystore::{Keystore, KeystoreConfig};
use gadget_logging::setup_log;
use gadget_testing_utils::anvil::start_default_anvil_testnet;
use gadget_utils::evm::get_provider_http;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tangle_subxt::subxt_signer::bip39;
use tempfile::{tempdir, TempDir};

#[test]
fn test_cli_fs_key_generation() -> Result<()> {
    let temp_dir = tempdir()?;
    let output_path = temp_dir.path();

    for key_type in [
        KeyTypeId::Sr25519,
        KeyTypeId::Ed25519,
        KeyTypeId::Ecdsa,
        KeyTypeId::Bls381,
        KeyTypeId::Bn254,
    ]
    .iter()
    {
        println!("Testing key generation for: {:?}", key_type);
        let (public, secret) = generate_key(*key_type, Some(&output_path), None, true)?;
        assert!(!public.is_empty());
        assert!(secret.is_some());
        assert!(!secret.unwrap().is_empty());

        let keystore = Keystore::new(KeystoreConfig::new().fs_root(output_path))?;
        match key_type {
            KeyTypeId::Sr25519 => {
                keystore.first_local::<SpSr25519>()?;
            }
            KeyTypeId::Ed25519 => {
                keystore.first_local::<SpEd25519>()?;
            }
            KeyTypeId::Ecdsa => {
                keystore.first_local::<SpEcdsa>()?;
            }
            KeyTypeId::Bls381 => {
                keystore.first_local::<SpBls381>()?;
            }
            KeyTypeId::Bn254 => {
                keystore.first_local::<ArkBlsBn254>()?;
            }
            _ => unreachable!(),
        }
    }
    Ok(())
}

#[test]
fn test_cli_mem_key_generation() -> Result<()> {
    for key_type in [
        KeyTypeId::Sr25519,
        KeyTypeId::Ed25519,
        KeyTypeId::Ecdsa,
        KeyTypeId::Bls381,
        KeyTypeId::Bn254,
    ]
    .iter()
    {
        println!("Testing key generation for: {:?}", key_type);
        let (public, secret) = generate_key(*key_type, None::<&PathBuf>, None, true)?;
        assert!(!public.is_empty());
        assert!(secret.is_some());
        assert!(!secret.unwrap().is_empty());
    }
    Ok(())
}

#[test]
fn test_load_signer_from_env() -> Result<()> {
    color_eyre::install().unwrap_or(());
    let s = [1u8; 32];
    let secret = bip39::Mnemonic::from_entropy(&s[..])?.to_string();
    // Test with a valid mnemonic phrase
    env::set_var(SIGNER_ENV, secret);
    load_signer_from_env()?;

    // Test with a valid hex string
    env::set_var(
        SIGNER_ENV,
        "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    );
    load_signer_from_env()?;

    // Test with an invalid mnemonic phrase
    env::set_var(SIGNER_ENV, "invalid mnemonic phrase");
    assert!(load_signer_from_env().is_err());

    // Test with an invalid hex string
    env::set_var(SIGNER_ENV, "0xinvalidhexstring");
    assert!(load_signer_from_env().is_err());

    // Test when the SIGNER environment variable is not set
    env::remove_var(SIGNER_ENV);
    assert!(load_signer_from_env().is_err());
    Ok(())
}

#[test]
fn test_load_evm_signer_from_env() -> Result<()> {
    color_eyre::install().unwrap_or(());
    let s = [1u8; 32];
    let secret = bip39::Mnemonic::from_entropy(&s[..])?.to_string();
    // Test with a valid mnemonic phrase
    env::set_var(EVM_SIGNER_ENV, secret);
    load_evm_signer_from_env()?;

    // Test with a valid hex string
    env::set_var(
        EVM_SIGNER_ENV,
        "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    );
    load_evm_signer_from_env()?;

    // Test with an invalid mnemonic phrase
    env::set_var(EVM_SIGNER_ENV, "invalid mnemonic phrase");
    assert!(load_evm_signer_from_env().is_err());

    // Test with an invalid hex string
    env::set_var(EVM_SIGNER_ENV, "0xinvalidhexstring");
    assert!(load_evm_signer_from_env().is_err());

    // Test when the EVM_SIGNER environment variable is not set
    env::remove_var(EVM_SIGNER_ENV);
    assert!(load_evm_signer_from_env().is_err());

    Ok(())
}

#[tokio::test]
async fn test_deploy_nonlocal_on_anvil() -> Result<()> {
    setup_log();

    // Create a temporary directory for our test contract
    let temp_dir = TempDir::new()?;
    let contract_src_dir = temp_dir.path().join("src");
    let contract_out_dir = temp_dir.path().join("out");
    let contract_dir = temp_dir.path();
    fs::create_dir_all(&contract_src_dir)?;
    fs::create_dir_all(&contract_out_dir)?;

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
    string private extraData;
    event DataStored(string newData);
    event Initialized(string extraData);

    constructor(string memory initialData) {
        storedData = initialData;
    }

    function initialize(string memory initExtraData) public {
        extraData = initExtraData;
        emit Initialized(initExtraData);
    }

    function set(string memory newData) public {
        storedData = newData;
        emit DataStored(newData);
    }

    function get() public view returns (string memory) {
        return storedData;
    }

    function getExtraData() public view returns (string memory) {
        return extraData;
    }
}
"#;

    fs::write(
        contract_src_dir.join("SimpleStorage.sol"),
        second_contract_content,
    )?;

    // Create foundry.toml
    let foundry_content = r#"[profile.default]
src = 'src'
out = 'out'
libs = ['lib']
evm_version = 'shanghai'"#;
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
        ordered_deployment: true,
    };

    // Build the contracts in temporary directory
    let build_output = Command::new("forge")
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
    let test_contract_address = contract_addresses
        .iter()
        .find(|(key, _value)| key.contains("TestContract"))
        .map(|(_key, value)| value.clone())
        .expect("Could not find TestContract in deployed contracts");
    let simple_storage_address = contract_addresses
        .iter()
        .find(|(key, _value)| key.contains("SimpleStorage"))
        .map(|(_key, value)| value.clone())
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

    // Verify the contract data using contract instance
    let simple_storage_address = contract_addresses
        .iter()
        .find(|(key, _value)| key.contains("SimpleStorage"))
        .map(|(_key, value)| value.clone())
        .expect("Could not find SimpleStorage in deployed contracts");
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

    let get_result = simple_storage_contract
        .function("getExtraData", &[])
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
    println!("getExtraData: {}", get_result_value);

    Ok(())
}
