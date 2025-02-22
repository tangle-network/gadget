use crate::keys::{generate_key, import_key};
use alloy_primitives::Address;
use color_eyre::Result;
use dialoguer::{Confirm, Input, Select};
use gadget_config::supported_chains::SupportedChains;
use gadget_config::Protocol;
use gadget_crypto::k256::K256Ecdsa;
use gadget_crypto::KeyTypeId;
use gadget_keystore::backends::Backend;
use gadget_keystore::{Keystore, KeystoreConfig};
use gadget_logging::info;
use gadget_std::fs;
use gadget_std::path::Path;
use gadget_std::process::Command;
use gadget_std::str::FromStr;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EigenlayerDeployOpts {
    /// The RPC URL to connect to
    pub(crate) rpc_url: String,
    /// Path to the contracts, defaults to `"./contracts"`
    pub(crate) contracts_path: String,
    /// Optional constructor arguments for contracts, keyed by contract name
    pub(crate) constructor_args: Option<HashMap<String, Vec<String>>>,
    /// Whether to deploy contracts in an interactive ordered manner
    pub(crate) ordered_deployment: bool,
    /// The type of the target chain
    pub(crate) chain: SupportedChains,
    /// The path to the keystore
    pub(crate) keystore_path: String,
}

impl EigenlayerDeployOpts {
    pub fn new(
        rpc_url: String,
        contracts_path: Option<String>,
        ordered_deployment: bool,
        chain: SupportedChains,
        keystore_path: Option<impl AsRef<Path>>,
    ) -> Self {
        let keystore_path = if keystore_path.is_none()
            && chain == SupportedChains::LocalTestnet
            && (rpc_url.contains("127.0.0.1") || rpc_url.contains("localhost"))
        {
            // For local testnet with no specified keystore, use a temporary directory
            let temp_dir = tempfile::tempdir()
                .expect("Failed to create temporary directory")
                .into_path();
            temp_dir.to_string_lossy().to_string()
        } else {
            keystore_path
                .map(|p| p.as_ref().to_string_lossy().to_string())
                .unwrap_or_else(|| "./keystore".to_string())
        };

        Self {
            rpc_url,
            contracts_path: contracts_path.unwrap_or_else(|| "./contracts".to_string()),
            constructor_args: None,
            ordered_deployment,
            chain,
            keystore_path,
        }
    }

    fn get_private_key(&self) -> Result<String> {
        let mut config = KeystoreConfig::new();
        // Check if keystore exists and create it if it doesn't
        if !Path::new(&self.keystore_path).exists() {
            std::fs::create_dir_all(&self.keystore_path)?;
        }
        config = config.fs_root(&self.keystore_path);
        let keystore = Keystore::new(config)?;

        if (self.rpc_url.contains("127.0.0.1") || self.rpc_url.contains("localhost"))
            && self.chain == SupportedChains::LocalTestnet
        {
            Ok("0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string())
        } else {
            // Try to get the ECDSA key from the keystore
            let keys = keystore.list_local::<K256Ecdsa>()?;
            if keys.is_empty() {
                println!(
                    "No ECDSA key found at {}. Let's set one up.",
                    self.keystore_path
                );
                let keys = crate::keys::prompt_for_keys(vec![KeyTypeId::Ecdsa])?;
                let (key_type, secret) = keys
                    .first()
                    .ok_or(color_eyre::eyre::eyre!("No ECDSA key found in keystore."))?;
                let private_key = secret.clone();
                let _public = crate::keys::import_key(
                    Protocol::Eigenlayer,
                    *key_type,
                    secret,
                    Path::new(&self.keystore_path),
                )?;
                return Ok(private_key);
            }

            Err(color_eyre::eyre::eyre!("No ECDSA key found in keystore. Please add one using 'cargo tangle key import' or set EIGENLAYER_PRIVATE_KEY environment variable"))
        }
    }
}

/// Initializes the test keystore with Anvil's Account 0. Generating the `./test-keystore` directory if it doesn't exist
///
/// Returns the path to the Temporary Directory, which must be kept alive as long as the keystore needs to be accessed.
pub fn initialize_test_keystore() -> Result<()> {
    // For local testnet with no specified keystore, use a temporary directory
    let keystore_path = Path::new("./test-keystore");
    let mut config = KeystoreConfig::new();
    if !keystore_path.exists() {
        fs::create_dir_all(&keystore_path)?;
    }
    config = config.fs_root(&keystore_path);
    let _keystore = Keystore::new(config)?;
    // TODO: Add support for Tangle here, taking the protocol as an input and controlling the key type and key input(s)
    import_key(
        Protocol::Eigenlayer,
        KeyTypeId::Ecdsa,
        "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
        &keystore_path,
    )?;
    generate_key(
        KeyTypeId::Bn254,
        Some(&keystore_path),
        None,
        false,
    )?;
    Ok(())
}

fn parse_contract_path(contract_path: &str) -> Result<String> {
    let path = Path::new(contract_path);

    // Check if the file has a .sol extension
    if path.extension().and_then(|ext| ext.to_str()) != Some("sol") {
        return Err(color_eyre::eyre::eyre!(
            "Contract file must have a .sol extension"
        ));
    }

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| color_eyre::eyre::eyre!("Invalid contract file name"))?;

    let contract_name = file_name.trim_end_matches(".sol");

    // Reconstruct the path with the contract name appended
    let mut new_path = path.to_path_buf();
    new_path.set_file_name(file_name); // Ensure we keep the .sol extension

    let formatted_path = new_path
        .to_str()
        .ok_or_else(|| color_eyre::eyre::eyre!("Failed to convert path to string"))?;

    Ok(format!("{}:{}", formatted_path, contract_name))
}

fn find_contract_files(contracts_path: &str) -> Result<Vec<String>> {
    let path = Path::new(contracts_path);
    if !path.exists() {
        return Err(color_eyre::eyre::eyre!(
            "Contracts path does not exist: {}",
            contracts_path
        ));
    }

    let mut contract_files = Vec::new();
    let src_path = path.join("src");

    if src_path.exists() {
        for entry in fs::read_dir(src_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "sol") {
                if let Some(path_str) = path.to_str() {
                    contract_files.push(path_str.to_string());
                }
            }
        }
    }

    if contract_files.is_empty() {
        return Err(color_eyre::eyre::eyre!(
            "No Solidity contract files found in {}/src",
            contracts_path
        ));
    }

    Ok(contract_files)
}

fn select_next_contract(available_contracts: &[String]) -> Result<String> {
    if available_contracts.is_empty() {
        return Err(color_eyre::eyre::eyre!("No contracts available to deploy"));
    }

    if available_contracts.len() == 1 {
        println!(
            "\nOnly one contract available to deploy: {}, deploying it now...\n",
            available_contracts[0]
        );
        return Ok(available_contracts[0].clone());
    }

    println!("\nAvailable contracts to deploy:");
    let selection = Select::new()
        .with_prompt("Select the contract to deploy (use arrow keys ↑↓)")
        .items(available_contracts)
        .default(0)
        .interact()?;

    println!(
        "\nNow deploying contract: {} Please wait...\n",
        available_contracts[selection]
    );

    Ok(available_contracts[selection].clone())
}

fn get_constructor_args(
    contract_json: &Value,
    contract_name: &str,
    provided_args: &Option<HashMap<String, Vec<String>>>,
) -> Option<Vec<String>> {
    // Find the constructor in the ABI
    let abi = contract_json.get("abi")?.as_array()?;
    let constructor = abi
        .iter()
        .find(|item| item.get("type").and_then(|t| t.as_str()) == Some("constructor"))?;

    // Get constructor inputs
    let inputs = constructor.get("inputs")?.as_array()?;
    if inputs.is_empty() {
        return None;
    }

    let contract_map_name = contract_name.rsplit(':').next().unwrap_or_default();

    // If we have pre-provided arguments for this contract, use those
    if let Some(args_map) = provided_args {
        if let Some(args) = args_map.get(contract_map_name) {
            if args.len() == inputs.len() {
                return Some(args.clone());
            }
        }
    }

    info!(
        "Contract '{}' requires constructor arguments:",
        contract_name
    );

    // For each input parameter, prompt the user for a value
    let mut args = Vec::new();
    for input in inputs {
        let name = input.get("name")?.as_str()?;
        let type_str = input.get("type")?.as_str()?;

        info!("Parameter '{}' of type '{}'", name, type_str);

        let value: String = Input::new()
            .with_prompt(format!("Enter value for {} ({})", name, type_str))
            .interact()
            .ok()?;

        args.push(value);
    }

    Some(args)
}

fn get_function_args_from_abi(
    contract_json: &Value,
    function_name: &str,
) -> Option<Vec<(String, String)>> {
    contract_json
        .get("abi")
        .and_then(|abi| abi.as_array())
        .and_then(|abi_array| {
            abi_array.iter().find(|func| {
                func.get("type").and_then(|t| t.as_str()) == Some("function")
                    && func.get("name").and_then(|n| n.as_str()) == Some(function_name)
            })
        })
        .and_then(|function| {
            function.get("inputs").and_then(|inputs| {
                inputs.as_array().map(|input_array| {
                    input_array
                        .iter()
                        .filter_map(|input| {
                            let name = input.get("name").and_then(|n| n.as_str())?;
                            let type_str = input.get("type").and_then(|t| t.as_str())?;
                            Some((name.to_string(), type_str.to_string()))
                        })
                        .collect()
                })
            })
        })
}

fn build_function_signature(function_name: &str, args: &[(String, String)]) -> String {
    let args_str = args
        .iter()
        .map(|(_, type_str)| type_str.as_str())
        .collect::<Vec<_>>()
        .join(",");
    format!("{}({})", function_name, args_str)
}

async fn initialize_contract_if_needed(
    opts: &EigenlayerDeployOpts,
    contract_json: &Value,
    contract_name: &str,
    contract_address: Address,
) -> Result<()> {
    // Check if contract has an initialize function
    if let Some(init_args) = get_function_args_from_abi(contract_json, "initialize") {
        info!("Contract {} is initializable", contract_name);

        let should_initialize = Confirm::new()
            .with_prompt(format!("Do you want to initialize {}?", contract_name))
            .default(false)
            .interact()?;

        if should_initialize {
            info!("Collecting initialization arguments...");
            let mut init_values = Vec::new();

            for (arg_name, arg_type) in &init_args {
                let prompt = format!("Enter value for {} (type: {})", arg_name, arg_type);
                let value: String = Input::new().with_prompt(&prompt).interact()?;

                // Format the value based on its type
                let formatted_value = if arg_type == "string" || arg_type.contains("bytes") {
                    format!("\"{}\"", value)
                } else {
                    value
                };

                init_values.push(formatted_value);
            }

            let function_sig = build_function_signature("initialize", &init_args);

            // First generate the calldata using cast calldata
            let calldata_cmd = format!(
                "cast calldata \"{}\" {}",
                function_sig,
                init_values.join(" ")
            );

            info!("Generating calldata: {}", calldata_cmd);

            let calldata_output = Command::new("sh").arg("-c").arg(&calldata_cmd).output()?;

            if !calldata_output.status.success() {
                return Err(color_eyre::eyre::eyre!(
                    "Failed to generate calldata: {}",
                    String::from_utf8_lossy(&calldata_output.stderr)
                ));
            }

            let calldata = String::from_utf8_lossy(&calldata_output.stdout)
                .trim()
                .to_string();
            info!("Generated calldata: {}", calldata);

            // Get the from address from the private key
            let from_cmd = format!(
                "cast wallet address --private-key {}",
                opts.get_private_key()?
            );

            let from_output = Command::new("sh").arg("-c").arg(&from_cmd).output()?;

            if !from_output.status.success() {
                return Err(color_eyre::eyre::eyre!(
                    "Failed to get from address: {}",
                    String::from_utf8_lossy(&from_output.stderr)
                ));
            }

            let from_address = String::from_utf8_lossy(&from_output.stdout)
                .trim()
                .to_string();

            // Construct the transaction parameters
            let tx_params = format!(
                "{{\"from\":\"{}\",\"to\":\"{}\",\"data\":\"{}\"}}",
                from_address, contract_address, calldata
            );

            // Send the transaction using eth_sendTransaction
            let command_str = format!(
                "cast rpc --rpc-url {} eth_sendTransaction '{}'",
                opts.rpc_url, tx_params
            );

            info!("Running command: {}", command_str);

            let mut cmd = Command::new("sh");
            cmd.arg("-c").arg(&command_str);

            let output = cmd.output()?;
            if !output.status.success() {
                return Err(color_eyre::eyre::eyre!(
                    "Failed to initialize contract: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }

            info!("Successfully initialized {}", contract_name);
        } else {
            info!("Skipping initialization of {}", contract_name);
        }
    }

    Ok(())
}

async fn deploy_single_contract(
    opts: &EigenlayerDeployOpts,
    contract_path: &str,
) -> Result<(String, Address)> {
    let contract_name = parse_contract_path(contract_path)?;
    info!("Deploying contract: {}", contract_name);

    let contract_output = contract_name.rsplit('/').next().ok_or_else(|| {
        color_eyre::eyre::eyre!("Failed to get contract output from path: {}", contract_name)
    })?;
    let contract_output = contract_output.replace(':', "/");
    info!("Contract output: {}", contract_output);

    // Read the contract's JSON artifact to check for constructor args
    let out_dir = Path::new(&opts.contracts_path).join("out");
    info!("Out directory: {}", out_dir.display());
    let json_path = out_dir.join(format!("{}.json", contract_output));
    info!("Contract JSON path: {}", json_path.display());

    // Read and parse the contract JSON
    let json_content = fs::read_to_string(&json_path)?;
    let contract_json: Value = serde_json::from_str(&json_content)?;

    // Build the forge create command as a single string
    let mut cmd_str = format!(
        "forge create {} --rpc-url {} --private-key {} --broadcast --evm-version shanghai --out {}",
        contract_name,
        opts.rpc_url,
        opts.get_private_key()?,
        Path::new(&opts.contracts_path).join("out").display()
    );

    if let Some(args) = get_constructor_args(&contract_json, &contract_name, &opts.constructor_args)
    {
        if !args.is_empty() {
            cmd_str.push_str(" --constructor-args");
            for value in args {
                // Quote the value if it's not already quoted
                let formatted_value = if !value.starts_with('"') {
                    format!("\"{}\"", value)
                } else {
                    value
                };
                cmd_str.push_str(&format!(" {}", formatted_value));
            }
        }
    }

    // Execute the command through sh -c
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(&cmd_str);

    let output = cmd.output()?;

    if !output.status.success() {
        return Err(color_eyre::eyre::eyre!(
            "Failed to deploy contract: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // Try to find address in stdout first, then stderr if not found
    let address = extract_address_from_output(output.stdout.clone())
        .or_else(|_| extract_address_from_output(output.stderr.clone()))
        .map_err(|_| {
            color_eyre::eyre::eyre!("Failed to find contract address in deployment output")
        })?;

    info!("Successfully deployed {} at {}", contract_name, address);

    // Check for initialization
    initialize_contract_if_needed(opts, &contract_json, &contract_name, address).await?;

    Ok((contract_name, address))
}

pub async fn deploy_avs_contracts(opts: &EigenlayerDeployOpts) -> Result<HashMap<String, Address>> {
    let mut deployed_addresses = HashMap::new();
    let contract_files = find_contract_files(&opts.contracts_path)?;

    if opts.ordered_deployment {
        println!("Starting ordered deployment of contracts...");

        let mut remaining_contracts = contract_files.clone();
        while !remaining_contracts.is_empty() {
            let selected_contract = select_next_contract(&remaining_contracts)?;
            println!("Selected contract: {}", selected_contract);

            let (contract_name, address) = deploy_single_contract(opts, &selected_contract).await?;
            deployed_addresses.insert(contract_name, address);

            // Remove the deployed contract from remaining contracts
            remaining_contracts.retain(|c| c != &selected_contract);
        }
    } else {
        println!("Finding contracts to deploy...");

        for contract_path in contract_files {
            let (contract_name, address) = deploy_single_contract(opts, &contract_path).await?;
            deployed_addresses.insert(contract_name, address);
        }
    }

    Ok(deployed_addresses)
}

pub async fn deploy_to_eigenlayer(opts: EigenlayerDeployOpts) -> Result<()> {
    println!("Beginning contract deployment");
    let addresses = deploy_avs_contracts(&opts).await?;
    println!("Successfully deployed contracts:");
    for (contract, address) in addresses {
        println!("{}: {}", contract, address);
    }
    Ok(())
}

pub fn extract_address_from_output(output: Vec<u8>) -> Result<Address> {
    let output = String::from_utf8_lossy(&output);
    info!("Attempting to extract address from output:\n{}", output);

    // Possible patterns to search for deployed address
    let patterns = [
        "Deployed to:",
        "Contract Address:",
        "Deployed at:",
        "at address:",
    ];

    for pattern in patterns {
        if let Some(line) = output.lines().find(|line| line.contains(pattern)) {
            println!("Found matching line with pattern '{}': {}", pattern, line);

            // Try to extract address
            let addr_str = line
                .split(pattern)
                .last()
                .and_then(|s| s.split_whitespace().next())
                .or_else(|| line.split_whitespace().last());

            if let Some(addr) = addr_str {
                println!("Found potential address: {}", addr);
                if let Ok(address) = Address::from_str(addr) {
                    println!("Successfully parsed address: {}", address);
                    return Ok(address);
                }
            }
        }
    }

    // If we get here, we couldn't find a valid address
    Err(color_eyre::eyre::eyre!(
        "Failed to find or parse contract address in output"
    ))
}
