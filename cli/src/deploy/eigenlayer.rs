use alloy_network::Network;
use alloy_provider::{Provider, ProviderBuilder, RootProvider};
use alloy_rpc_types_eth::{BlockNumberOrTag, TransactionRequest};
use alloy_signer_local::PrivateKeySigner;
use alloy_transport::BoxTransport;
use color_eyre::Result;
use gadget_logging::info;
use gadget_std::env;
use gadget_std::path::Path;
use gadget_std::path::PathBuf;
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
    /// Path to the contract to deploy (e.g., "path/to/MyContract.sol")
    pub contract_path: String,
}

impl EigenlayerDeployOpts {
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

pub async fn deploy_local(
    opts: &EigenlayerDeployOpts,
    provider: RootProvider<BoxTransport>,
    wallet: PrivateKeySigner,
) -> Result<()> {
    info!("Deploying contracts to local network using Alloy...");

    // Get deployment parameters
    let nonce = provider.get_transaction_count(wallet.address()).await?;

    // TODO: local deployment logic

    Ok(())
}

pub async fn deploy_nonlocal(opts: &EigenlayerDeployOpts) -> Result<()> {
    info!("Deploying contracts using Forge...");

    // let network_arg = match opts.network {
    //     NetworkTarget::Testnet => "--network testnet",
    //     NetworkTarget::Mainnet => "--network mainnet",
    //     NetworkTarget::Local => "--network testnet",
    // };

    // let etherscan_key = opts
    //     .get_etherscan_key()?
    //     .map(|key| format!("--etherscan-api-key {}", key))
    //     .unwrap_or_default();

    let private_key = opts.get_private_key()?;

    let contract_path = parse_contract_path(&opts.contract_path)?;
    info!("Contract path: {}", contract_path);

    // Construct the forge create command
    let mut cmd = Command::new("forge");
    cmd.arg("create")
        .arg(contract_path)
        .arg("--rpc-url")
        .arg(&opts.rpc_url)
        .arg("--private-key")
        .arg(&private_key);
    // .arg("--etherscan-api-key").arg("$ETHERSCAN_API_KEY")
    // .arg(network_arg)
    // .arg("--verify");

    info!("Running command: {:?}", cmd);

    // if !etherscan_key.is_empty() {
    //     cmd.arg(&etherscan_key);
    // }

    // Execute the command
    info!("Executing forge create command...");
    let output = cmd.output().unwrap();

    // info!("Standard Output: {}", String::from_utf8_lossy(&output.stdout));
    // info!("Standard Error: {}", String::from_utf8_lossy(&output.stderr));

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        info!("Contract deployed successfully. Output:\n{}", stdout);
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(color_eyre::eyre::eyre!(
            "Deployment failed. Error:\n{}",
            stderr
        ));
    }

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

pub async fn deploy_to_eigenlayer(opts: EigenlayerDeployOpts) -> Result<()> {
    info!("Starting Eigenlayer deployment process...");

    match opts.network {
        NetworkTarget::Local => {
            let provider = ProviderBuilder::new()
                .on_http((&opts.rpc_url).parse()?)
                .boxed();

            let wallet = PrivateKeySigner::from_str(&opts.get_private_key()?)?;
            deploy_local(&opts, provider, wallet).await?;
        }
        NetworkTarget::Testnet | NetworkTarget::Mainnet => {
            deploy_nonlocal(&opts).await?;
        }
    }

    info!("Deployment completed successfully!");
    Ok(())
}
