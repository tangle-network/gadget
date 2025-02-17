use alloy_primitives::Address;
use alloy_provider::{Provider, ProviderBuilder, RootProvider};
use alloy_signer_local::PrivateKeySigner;
use alloy_transport::BoxTransport;
use color_eyre::Result;
use gadget_logging::info;
use gadget_std::env;
use gadget_std::fs;
use gadget_std::path::Path;
use gadget_std::process::Command;
use gadget_std::str::FromStr;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkTarget {
    Local,
    Testnet,
    Mainnet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EigenlayerDeployOpts {
    /// The target network for deployment
    pub network: NetworkTarget,
    /// The RPC URL to connect to
    pub rpc_url: String,
    /// Path to the contracts, defaults to `"./contracts"`
    pub contracts_path: String,
}

impl EigenlayerDeployOpts {
    pub fn new(network: NetworkTarget, rpc_url: String) -> Self {
        Self {
            network,
            rpc_url,
            contracts_path: "./contracts".to_string(),
        }
    }

    fn get_private_key(&self) -> Result<String> {
        match self.network {
            NetworkTarget::Local => Ok(
                "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string(),
            ), // Default Anvil private key
            _ => env::var("EIGENLAYER_PRIVATE_KEY").map_err(|_| {
                color_eyre::eyre::eyre!("EIGENLAYER_PRIVATE_KEY environment variable not set")
            }),
        }
    }

    fn get_etherscan_key(&self) -> Result<Option<String>> {
        match self.network {
            NetworkTarget::Local => Ok(None),
            _ => env::var("ETHERSCAN_API_KEY").map(Some).map_err(|_| {
                color_eyre::eyre::eyre!("ETHERSCAN_API_KEY environment variable not set")
            }),
        }
    }
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
        return Err(color_eyre::eyre::eyre!("Contracts path does not exist: {}", contracts_path));
    }

    let mut contract_files = Vec::new();
    let src_path = path.join("src");
    
    if src_path.exists() {
        for entry in fs::read_dir(src_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "sol") {
                if let Some(path_str) = path.to_str() {
                    contract_files.push(path_str.to_string());
                }
            }
        }
    }

    if contract_files.is_empty() {
        return Err(color_eyre::eyre::eyre!("No Solidity contract files found in {}/src", contracts_path));
    }

    Ok(contract_files)
}

pub async fn deploy_avs_contracts(opts: &EigenlayerDeployOpts) -> Result<Vec<Address>> {
    info!("Finding contracts to deploy...");
    
    let contract_files = find_contract_files(&opts.contracts_path)?;
    let private_key = opts.get_private_key()?;
    let mut deployed_addresses = Vec::new();

    for contract_path in contract_files {
        info!("Deploying contract: {}", contract_path);

        let contract_name = parse_contract_path(&contract_path)?;

        // Construct the forge create command
        let mut cmd = Command::new("forge");
        cmd.arg("create")
            .arg(&contract_name)
            .arg("--rpc-url")
            .arg(&opts.rpc_url)
            .arg("--private-key")
            .arg(&private_key);

        info!("Running command: {:?}", cmd);

        // Execute the command
        let output = cmd.output()?;
        
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            info!("Contract deployed successfully. Output:\n{}", stdout);
            
            // Extract deployed address from output
            if let Some(address) = stdout
                .lines()
                .find(|line| line.contains("Deployed to:"))
                .and_then(|line| line.split_whitespace().last())
                .and_then(|addr_str| Address::from_str(addr_str).ok())
            {
                deployed_addresses.push(address);
                info!("Contract deployed to: {}", address);
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(color_eyre::eyre::eyre!(
                "Failed to deploy contract {}: {}",
                contract_path,
                stderr
            ));
        }
    }

    Ok(deployed_addresses)
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
