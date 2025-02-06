use alloy_network::Network;
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types_eth::{BlockNumberOrTag, TransactionRequest};
use alloy_signer_local::LocalWallet;
use color_eyre::Result;
use gadget_logging::info;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// Path to the AVS/blueprint configuration file
    pub config_path: PathBuf,
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

async fn deploy_local(
    opts: &EigenlayerDeployOpts,
    provider: Provider,
    wallet: LocalWallet,
) -> Result<()> {
    info!("Deploying contracts to local network using Alloy...");

    // Get deployment parameters
    let nonce = provider
        .get_transaction_count(wallet.address(), Some(BlockNumberOrTag::Latest))
        .await?;

    // TODO: local deployment logic

    Ok(())
}

async fn deploy_nonlocal(opts: &EigenlayerDeployOpts) -> Result<()> {
    info!("Deploying contracts using Forge...");

    let network_arg = match opts.network {
        NetworkTarget::Testnet => "--network testnet",
        NetworkTarget::Mainnet => "--network mainnet",
        _ => unreachable!(),
    };

    // Run forge create for each contract
    let etherscan_key = opts
        .get_etherscan_key()?
        .map(|key| format!("--etherscan-api-key {}", key))
        .unwrap_or_default();

    let private_key = opts.get_private_key()?;

    // TODO: Add forge create command
    // forge create src/ContractName.sol:ContractName
    //   --rpc-url {opts.rpc_url}
    //   --private-key {private_key}
    //   {network_arg}
    //   {etherscan_key}
    //   --verify

    Ok(())
}

pub async fn deploy_to_eigenlayer(opts: EigenlayerDeployOpts) -> Result<()> {
    info!("Starting Eigenlayer deployment process...");

    match opts.network {
        NetworkTarget::Local => {
            let provider = ProviderBuilder::new().url(&opts.rpc_url).build()?;

            let wallet = LocalWallet::from_private_key_str(&opts.get_private_key()?)?;
            deploy_local(&opts, provider, wallet).await?;
        }
        NetworkTarget::Testnet | NetworkTarget::Mainnet => {
            deploy_nonlocal(&opts).await?;
        }
    }

    info!("Deployment completed successfully!");
    Ok(())
}
