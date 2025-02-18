use alloy_primitives::Address;
use color_eyre::Result;
use dialoguer::Input;
use gadget_logging::info;
use gadget_std::env;
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
}

impl EigenlayerDeployOpts {
    pub fn new(rpc_url: String, contracts_path: Option<String>, ordered_deployment: bool) -> Self {
        Self {
            rpc_url,
            contracts_path: contracts_path.unwrap_or_else(|| "./contracts".to_string()),
            constructor_args: None,
            ordered_deployment,
        }
    }

    fn get_private_key(&self) -> Result<String> {
        if self.rpc_url.contains("127.0.0.1") || self.rpc_url.contains("localhost") {
            Ok("0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string())
        // Default Anvil private key
        } else {
            env::var("EIGENLAYER_PRIVATE_KEY").map_err(|_| {
                color_eyre::eyre::eyre!("EIGENLAYER_PRIVATE_KEY environment variable not set")
            })
        }
    }

    // TODO: Implement verification flow using etherscan
    // fn get_etherscan_key(&self) -> Result<Option<String>> {
    //     if self.rpc_url.contains("127.0.0.1") || self.rpc_url.contains("localhost") {
    //         Ok(None)
    //     } else {
    //         env::var("ETHERSCAN_API_KEY").map(Some).map_err(|_| {
    //             color_eyre::eyre::eyre!("ETHERSCAN_API_KEY environment variable not set")
    //         })
    //     }
    // }
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

    println!("\nAvailable contracts to deploy:");
    for (i, contract) in available_contracts.iter().enumerate() {
        println!("{}: {}", i + 1, contract);
    }

    let selection: usize = Input::new()
        .with_prompt("Select the contract to deploy (enter the number)")
        .validate_with(|input: &usize| {
            if *input > 0 && *input <= available_contracts.len() {
                Ok(())
            } else {
                Err("Invalid selection")
            }
        })
        .interact()?;

    Ok(available_contracts[selection - 1].clone())
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

fn get_constructor_args_as_cli_args(
    contract_path: &str,
    contract_name: &str,
    provided_args: &Option<HashMap<String, Vec<String>>>,
) -> Result<Vec<String>> {
    if let Some(args) = get_constructor_args(
        &serde_json::from_str(contract_path)?,
        contract_name,
        provided_args,
    ) {
        Ok(args
            .into_iter()
            .map(|arg| format!("\"{}\"", arg))
            .collect::<Vec<_>>())
    } else {
        Ok(Vec::new())
    }
}

pub async fn deploy_avs_contracts(opts: &EigenlayerDeployOpts) -> Result<Vec<Address>> {
    let mut deployed_addresses = Vec::new();
    let mut contract_files = find_contract_files(&opts.contracts_path)?;

    if opts.ordered_deployment {
        info!("Starting ordered deployment of contracts...");

        let mut remaining_contracts = contract_files.clone();
        while !remaining_contracts.is_empty() {
            let selected_contract = select_next_contract(&remaining_contracts)?;
            info!("Selected contract: {}", selected_contract);

            // Deploy the selected contract
            let contract_path = selected_contract.clone();
            let contract_name = parse_contract_path(&contract_path)?;

            let contract_output = contract_name.rsplit('/').next().ok_or_else(|| {
                color_eyre::eyre::eyre!(
                    "Failed to get contract output from path: {}",
                    contract_name
                )
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

            info!("Deploying {}", contract_name);

            let mut cmd = Command::new("forge");
            cmd.args([
                "create",
                &contract_name,
                "--rpc-url",
                &opts.rpc_url,
                "--private-key",
                &opts.get_private_key()?,
            ]);

            if let Some(args) =
                get_constructor_args(&contract_json, &contract_name, &opts.constructor_args)
            {
                if !args.is_empty() {
                    cmd.arg("--constructor-args");
                    for value in args {
                        cmd.arg(value);
                    }
                }
            }
            let output = cmd.output()?;

            if !output.status.success() {
                return Err(color_eyre::eyre::eyre!(
                    "Failed to deploy contract: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }

            let address = extract_address_from_output(output.stdout)?;
            deployed_addresses.push(address);

            // Remove the deployed contract from remaining contracts
            remaining_contracts.retain(|c| c != &selected_contract);

            info!("Successfully deployed {} at {}", contract_name, address);
        }
    } else {
        info!("Finding contracts to deploy...");

        for contract_path in contract_files {
            let contract_name = parse_contract_path(&contract_path)?;
            info!("Deploying contract: {}", contract_name);

            let contract_output = contract_name.rsplit('/').next().ok_or_else(|| {
                color_eyre::eyre::eyre!(
                    "Failed to get contract output from path: {}",
                    contract_name
                )
            })?;
            let contract_output = contract_output.replace(':', "/");

            // Read the contract's JSON artifact to check for constructor args
            let out_dir = Path::new(&opts.contracts_path).join("out");
            info!("Out directory: {}", out_dir.display());
            let json_path = out_dir.join(format!("{}.json", contract_output));
            info!("Contract JSON path: {}", json_path.display());

            // Read and parse the contract JSON
            let json_content = fs::read_to_string(&json_path)?;
            let contract_json: Value = serde_json::from_str(&json_content)?;

            // Construct the forge create command
            let mut cmd = Command::new("forge");
            cmd.arg("create")
                .arg(&contract_name)
                .arg("--rpc-url")
                .arg(&opts.rpc_url)
                .arg("--private-key")
                .arg(&opts.get_private_key()?);

            // If the contract has constructor args, get them from the user or use provided args
            if let Some(args) =
                get_constructor_args(&contract_json, &contract_name, &opts.constructor_args)
            {
                if !args.is_empty() {
                    cmd.arg("--constructor-args");
                    for value in args {
                        cmd.arg(value);
                    }
                }
            }

            info!("Running command: {:?}", cmd);

            // Execute the command
            let output = cmd.output()?;

            if output.status.success() {
                let address = extract_address_from_output(output.stdout)?;
                info!("Successfully deployed {} at {}", contract_name, address);
                deployed_addresses.push(address);
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(color_eyre::eyre::eyre!(
                    "Failed to deploy contract {}: {}",
                    contract_path,
                    stderr
                ));
            }
        }

        return Ok(deployed_addresses);
    }

    Ok(deployed_addresses)
}

pub fn extract_address_from_output(output: Vec<u8>) -> Result<Address> {
    let output = String::from_utf8_lossy(&output);
    output
        .lines()
        .find(|line| line.contains("Deployed to:"))
        .and_then(|line| line.split_whitespace().last())
        .and_then(|addr_str| Address::from_str(addr_str).ok())
        .ok_or_else(|| color_eyre::eyre::eyre!("Failed to parse contract address"))
}

pub async fn deploy_to_eigenlayer(opts: EigenlayerDeployOpts) -> Result<()> {
    info!("Deploying contracts to EigenLayer...");
    let addresses = deploy_avs_contracts(&opts).await?;

    if addresses.is_empty() {
        info!("No contracts were deployed");
    } else {
        info!("Successfully deployed {} contracts:", addresses.len());
        for (i, address) in addresses.iter().enumerate() {
            info!("Contract {}: {}", i + 1, address);
        }
    }

    Ok(())
}
