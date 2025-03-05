use std::path::PathBuf;

use crate::deploy::tangle::{deploy_to_tangle, Opts};
use crate::keys::prompt_for_keys;
use cargo_tangle::anvil::start_default_anvil_testnet;
use cargo_tangle::create::BlueprintType;
#[cfg(feature = "eigenlayer")]
use cargo_tangle::deploy::eigenlayer::{deploy_to_eigenlayer, EigenlayerDeployOpts};
use cargo_tangle::run::eigenlayer::run_eigenlayer_avs;
use cargo_tangle::{create, deploy, keys};
use clap::{Parser, Subcommand};
use dialoguer::console::style;
use dotenv::from_path;
use gadget_config::{
    protocol::{EigenlayerContractAddresses, Protocol, ProtocolSettings, TangleInstanceSettings},
    supported_chains::SupportedChains,
    Error, GadgetConfiguration,
};
use gadget_crypto::KeyTypeId;
use gadget_std::env;
use tokio::signal;

/// Tangle CLI tool
#[derive(Parser, Debug)]
#[clap(
    bin_name = "cargo-tangle",
    version,
    propagate_version = true,
    arg_required_else_help = true
)]
struct Cli {
    #[command(flatten)]
    manifest: clap_cargo::Manifest,
    #[command(flatten)]
    features: clap_cargo::Features,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Blueprint subcommand
    #[command(visible_alias = "bp")]
    Blueprint {
        #[command(subcommand)]
        command: BlueprintCommands,
    },

    /// Key management commands
    #[command(visible_alias = "k")]
    Key {
        #[command(subcommand)]
        command: KeyCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum KeyCommands {
    /// Generate a new key
    #[command(visible_alias = "g")]
    Generate {
        /// The type of key to generate (sr25519, ed25519, ecdsa, bls381, bls377, bn254)
        #[arg(short = 't', long, value_enum)]
        key_type: KeyTypeId,
        /// The path to save the key to
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,
        /// The seed to use for key generation (hex format without 0x prefix)
        #[arg(long)]
        seed: Option<Vec<u8>>,
        /// Show the secret key in output
        #[arg(short = 'v', long)]
        show_secret: bool,
    },
    /// Import a key into the keystore
    #[command(visible_alias = "i")]
    Import {
        /// The type of key to import (sr25519, ed25519, ecdsa, bls381, bls377, bn254)
        #[arg(short = 't', long, value_enum)]
        key_type: Option<KeyTypeId>,
        /// The secret key to import (hex format without 0x prefix)
        #[arg(short = 'x', long)]
        secret: Option<String>,
        /// The path to the keystore
        #[arg(short = 'k', long)]
        keystore_path: PathBuf,
        /// The protocol you are generating keys for (Eigenlayer or Tangle). Only matters for some keys.
        #[arg(short = 'p', long, default_value = "tangle")]
        protocol: Protocol,
    },
    /// Export a key from the keystore
    #[command(visible_alias = "e")]
    Export {
        /// The type of key to export (sr25519, ed25519, ecdsa, bls381, bls377, bn254)
        #[arg(short = 't', long, value_enum)]
        key_type: KeyTypeId,
        /// The public key to export (hex format without 0x prefix)
        #[arg(short = 'p', long)]
        public: String,
        /// The path to the keystore
        #[arg(short = 'k', long)]
        keystore_path: PathBuf,
    },
    /// List all keys in the keystore
    #[command(visible_alias = "l")]
    List {
        /// The path to the keystore
        #[arg(short = 'k', long)]
        keystore_path: PathBuf,
    },
    /// Generate a new mnemonic phrase
    #[command(visible_alias = "m")]
    GenerateMnemonic {
        /// Number of words in the mnemonic (12, 15, 18, 21, or 24)
        #[arg(short = 'w', long, value_parser = clap::value_parser!(u32).range(12..=24))]
        word_count: Option<u32>,
    },
}

#[derive(Subcommand, Debug)]
pub enum BlueprintCommands {
    /// Create a new blueprint
    #[command(visible_alias = "c")]
    Create {
        /// The name of the blueprint
        #[arg(short = 'n', long, value_name = "NAME", env = "NAME")]
        name: String,

        #[command(flatten)]
        source: Option<create::Source>,

        #[command(flatten)]
        blueprint_type: Option<BlueprintType>,
    },

    /// Deploy a blueprint to the Tangle Network or Eigenlayer.
    #[command(visible_alias = "d")]
    Deploy {
        #[command(subcommand)]
        target: DeployTarget,
    },

    /// Run a gadget
    #[command(visible_alias = "r")]
    Run {
        /// The protocol to run (eigenlayer or tangle)
        #[arg(short = 'p', long, value_enum)]
        protocol: Protocol,

        /// The HTTP RPC endpoint URL (required)
        #[arg(short = 'u', long)]
        rpc_url: String,

        /// The keystore path (defaults to ./keystore)
        #[arg(short = 'k', long)]
        keystore_path: Option<PathBuf>,

        /// The path to the AVS binary
        ///
        /// If not provided, the binary will be built if possible
        #[arg(short = 'b', long)]
        binary_path: Option<PathBuf>,

        /// The network to connect to (local, testnet, mainnet)
        #[arg(short = 'w', long, default_value = "local")]
        network: String,

        /// The data directory path (defaults to ./data)
        #[arg(short = 'd', long)]
        data_dir: Option<PathBuf>,

        /// Optional bootnodes to connect to
        #[arg(short = 'n', long)]
        bootnodes: Option<Vec<String>>,

        /// Path to the protocol settings env file
        #[arg(short = 'f', long, default_value = "./settings.env")]
        settings_file: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
pub enum DeployTarget {
    /// Deploy to Tangle Network
    Tangle {
        /// HTTP RPC URL to use
        #[arg(
            long,
            value_name = "URL",
            default_value = "https://rpc.tangle.tools",
            env
        )]
        http_rpc_url: String,
        /// Tangle RPC URL to use
        #[arg(
            long,
            value_name = "URL",
            default_value = "wss://rpc.tangle.tools",
            env
        )]
        ws_rpc_url: String,
        /// The package to deploy (if the workspace has multiple packages).
        #[arg(short = 'p', long, value_name = "PACKAGE", env = "CARGO_PACKAGE")]
        package: Option<String>,
    },
    /// Deploy to Eigenlayer
    #[cfg(feature = "eigenlayer")]
    Eigenlayer {
        /// HTTP RPC URL to use
        #[arg(long, value_name = "URL", env, required_unless_present = "devnet")]
        rpc_url: Option<String>,
        /// Path to the contracts
        #[arg(long)]
        contracts_path: Option<String>,
        /// Whether to deploy contracts in an interactive ordered manner
        #[arg(long)]
        ordered_deployment: bool,
        /// Network to deploy to (local, testnet, mainnet)
        #[arg(short = 'w', long, default_value = "local")]
        network: String,
        /// Start a local devnet using Anvil (only valid with network=local)
        #[arg(long)]
        devnet: bool,
        /// The keystore path (defaults to ./keystore)
        #[arg(short = 'k', long)]
        keystore_path: Option<PathBuf>,
    },
}

#[tokio::main]
#[allow(clippy::needless_return)]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    init_tracing_subscriber();
    let args: Vec<String> = if std::env::args()
        .nth(1)
        .map(|x| x.eq("tangle"))
        .unwrap_or(false)
    {
        // since this runs as a cargo subcommand, we need to skip the first argument
        // to get the actual arguments for the subcommand
        std::env::args().skip(1).collect()
    } else {
        std::env::args().collect()
    };

    // Parse the CLI arguments
    let cli = Cli::parse_from(args);

    match cli.command {
        Commands::Blueprint { command } => match command {
            BlueprintCommands::Create {
                name,
                source,
                blueprint_type,
            } => {
                create::new_blueprint(name, source, blueprint_type)?;
            }
            BlueprintCommands::Deploy { target } => match target {
                DeployTarget::Tangle {
                    http_rpc_url,
                    ws_rpc_url,
                    package,
                } => {
                    let manifest_path = cli
                        .manifest
                        .manifest_path
                        .unwrap_or_else(|| PathBuf::from("Cargo.toml"));
                    let _ = deploy_to_tangle(Opts {
                        http_rpc_url,
                        ws_rpc_url,
                        manifest_path,
                        pkg_name: package,
                        signer: None,
                        signer_evm: None,
                    })
                    .await?;
                }
                #[cfg(feature = "eigenlayer")]
                DeployTarget::Eigenlayer {
                    rpc_url,
                    contracts_path,
                    ordered_deployment,
                    network,
                    devnet,
                    keystore_path,
                } => {
                    let build_status = gadget_std::process::Command::new("cargo")
                        .args(["build", "--release"])
                        .status()?;

                    if !build_status.success() {
                        return Err(color_eyre::Report::msg("Cargo build failed"));
                    }

                    // Validate that devnet is only used with local network
                    if devnet && network.to_lowercase() != "local" {
                        return Err(color_eyre::Report::msg(
                            "The --devnet flag can only be used with --network local",
                        ));
                    }

                    let chain = match network.to_lowercase().as_str() {
                        "local" => SupportedChains::LocalTestnet,
                        "testnet" => SupportedChains::Testnet,
                        "mainnet" => {
                            if rpc_url.as_ref().is_some_and(|url| {
                                url.contains("127.0.0.1") || url.contains("localhost")
                            }) {
                                SupportedChains::LocalMainnet
                            } else {
                                SupportedChains::Mainnet
                            }
                        }
                        _ => {
                            return Err(color_eyre::Report::msg(format!(
                                "Invalid network: {}",
                                network
                            )));
                        }
                    };

                    if chain == SupportedChains::LocalTestnet && devnet {
                        // Start local Anvil testnet
                        let (_container, http_endpoint, _ws_endpoint) =
                            start_default_anvil_testnet(true).await;

                        deploy::eigenlayer::initialize_test_keystore()?;

                        // Deploy to local devnet
                        deploy_to_eigenlayer(EigenlayerDeployOpts::new(
                            http_endpoint.clone(),
                            contracts_path,
                            ordered_deployment,
                            chain,
                            keystore_path,
                        ))
                        .await?;

                        // Keep the process running and show helpful instructions
                        println!("\n{}", style("Local Testnet Active").green().bold());
                        println!("\n{}", style("To run your AVS:").cyan().bold());
                        println!("{}", style("1. Open a new terminal window").dim());
                        println!(
                            "{}",
                            style("2. Set your AVS-specific environment variables:").dim()
                        );
                        println!(
                            "   {}",
                            style("# Your AVS may require specific environment variables from the deployment output above").dim()
                        );
                        println!(
                            "   {}",
                            style("# For example: TASK_MANAGER_ADDRESS=<address> or other contract addresses").dim()
                        );
                        println!("\n{}", style("3. Run your AVS with:").dim());
                        println!(
                            "   {}\n   {}\n   {}\n   {}",
                            style("cargo tangle blueprint run \\").yellow(),
                            style("  -p eigenlayer \\").yellow(),
                            style(format!("  -u {} \\", http_endpoint)).yellow(),
                            style("  --keystore-path ./test-keystore").yellow()
                        );
                        println!(
                            "\n{}",
                            style("The deployment variables above show all contract addresses you may need.").dim()
                        );
                        println!("{}", style("Press Ctrl+C to stop the testnet...").dim());
                        signal::ctrl_c().await?;
                        println!("{}", style("\nShutting down devnet...").yellow());
                    } else {
                        deploy_to_eigenlayer(EigenlayerDeployOpts::new(
                            rpc_url
                                .as_ref()
                                .map(|s| s.to_string())
                                .ok_or_else(|| {
                                    color_eyre::Report::msg(
                                        "The --rpc-url flag is required when deploying to a non-local network",
                                    )
                                })?,
                            contracts_path,
                            ordered_deployment,
                            chain,
                            keystore_path,
                        ))
                        .await?;
                    }
                }
            },
            BlueprintCommands::Run {
                protocol,
                rpc_url,
                keystore_path,
                binary_path,
                network,
                data_dir,
                bootnodes,
                settings_file,
            } => {
                let settings_file =
                    settings_file.unwrap_or_else(|| PathBuf::from("./settings.env"));
                if !settings_file.exists() {
                    return Err(color_eyre::Report::msg(format!(
                        "The --settings-file flag needs to be provided with a valid path, or the file {settings_file:?} needs to exist",
                    )));
                }
                let protocol_settings = load_protocol_settings(protocol, &settings_file)?;

                let chain = match network.to_lowercase().as_str() {
                    "local" => SupportedChains::LocalTestnet,
                    "testnet" => SupportedChains::Testnet,
                    "mainnet" => {
                        if rpc_url.contains("127.0.0.1") || rpc_url.contains("localhost") {
                            SupportedChains::LocalMainnet
                        } else {
                            SupportedChains::Mainnet
                        }
                    }
                    _ => {
                        return Err(color_eyre::Report::msg(format!(
                            "Invalid network: {}",
                            network
                        )));
                    }
                };

                let mut config = GadgetConfiguration::default();
                let ws_url = if let Some(stripped) = rpc_url.strip_prefix("http://") {
                    format!("ws://{}", stripped)
                } else if let Some(stripped) = rpc_url.strip_prefix("https://") {
                    format!("wss://{}", stripped)
                } else {
                    panic!("Invalid RPC URL format");
                };
                config.http_rpc_endpoint = rpc_url.clone();
                config.ws_rpc_endpoint = ws_url;
                let keystore_path = keystore_path.unwrap_or_else(|| PathBuf::from("./keystore"));
                if !keystore_path.exists() {
                    println!(
                        "Keystore not found at {}. Let's set up your keys.",
                        keystore_path.display()
                    );
                    let keys = prompt_for_keys(vec![KeyTypeId::Ecdsa])?;
                    std::fs::create_dir_all(&keystore_path)?;
                    for (key_type, key) in keys {
                        let key_path = keystore_path.join(format!("{:?}", key_type));
                        std::fs::write(key_path, key)?;
                    }
                }
                config.keystore_uri = keystore_path.to_string_lossy().to_string();
                config.data_dir = data_dir.or_else(|| Some(PathBuf::from("./data")));
                config.bootnodes = bootnodes
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|addr| addr.parse().ok())
                    .collect();
                config.protocol = protocol;
                config.protocol_settings = protocol_settings;
                config.test_mode = network == "local";

                match protocol {
                    Protocol::Eigenlayer => {
                        run_eigenlayer_avs(config, chain, binary_path).await?;
                    }
                    Protocol::Tangle => {
                        // Tangle implementation will go here
                        unimplemented!("Tangle protocol implementation not yet available");
                    }
                    _ => {
                        return Err(Error::UnsupportedProtocol(protocol.to_string()).into());
                    }
                }
            }
        },
        Commands::Key { command } => match command {
            KeyCommands::Generate {
                key_type,
                output,
                seed,
                show_secret,
            } => {
                let seed = seed.map(hex::decode).transpose()?;
                let (public, secret) =
                    keys::generate_key(key_type, output.as_ref(), seed.as_deref(), show_secret)?;

                eprintln!("Generated {:?} key:", key_type);
                eprintln!("Public key: {}", public);
                if show_secret || output.is_none() {
                    eprintln!("Private key: {}", secret.expect("Should exist"));
                }
            }
            KeyCommands::Import {
                key_type,
                secret,
                keystore_path,
                protocol,
            } => {
                if let Some(key_type) = key_type {
                    // If key_type is provided, require secret
                    let secret = secret.ok_or_else(|| {
                        color_eyre::eyre::eyre!("Secret key is required when key type is specified")
                    })?;
                    let public = keys::import_key(protocol, key_type, &secret, &keystore_path)?;
                    eprintln!("Imported {:?} key:", key_type);
                    eprintln!("Public key: {}", public);
                } else {
                    // If no key_type provided, use interactive prompt
                    let key_pairs = keys::prompt_for_keys(vec![])?;
                    for (key_type, secret) in key_pairs {
                        let public = keys::import_key(protocol, key_type, &secret, &keystore_path)?;
                        eprintln!("Imported {:?} key:", key_type);
                        eprintln!("Public key: {}", public);
                    }
                }
            }
            KeyCommands::Export {
                key_type,
                public,
                keystore_path,
            } => {
                let secret = keys::export_key(key_type, &public, &keystore_path)?;
                eprintln!("Exported {:?} key:", key_type);
                eprintln!("Public key: {}", public);
                eprintln!("Private key: {}", secret);
            }
            KeyCommands::List { keystore_path } => {
                let keys = keys::list_keys(&keystore_path)?;
                eprintln!("Keys in keystore:");
                for (key_type, public) in keys {
                    eprintln!("{:?}: {}", key_type, public);
                }
            }
            KeyCommands::GenerateMnemonic { word_count } => {
                let mnemonic = keys::generate_mnemonic(word_count)?;
                eprintln!("Generated mnemonic phrase:");
                eprintln!("{}", mnemonic);
                eprintln!("\nWARNING: Store this mnemonic phrase securely. It can be used to recover your keys.");
            }
        },
    }
    Ok(())
}

fn load_protocol_settings(
    protocol: Protocol,
    settings_file: &PathBuf,
) -> Result<ProtocolSettings, Error> {
    // Load environment variables from the settings file
    from_path(settings_file)
        .map_err(|e| Error::ConfigurationError(format!("Failed to load settings file: {}", e)))?;

    match protocol {
        Protocol::Eigenlayer => {
            let addresses = EigenlayerContractAddresses {
                allocation_manager_address: env::var("ALLOCATION_MANAGER_ADDRESS")
                    .map_err(|_| {
                        Error::ConfigurationError("Missing ALLOCATION_MANAGER_ADDRESS".into())
                    })?
                    .parse()
                    .map_err(|_| {
                        Error::ConfigurationError("Invalid ALLOCATION_MANAGER_ADDRESS".into())
                    })?,
                registry_coordinator_address: env::var("REGISTRY_COORDINATOR_ADDRESS")
                    .map_err(|_| {
                        Error::ConfigurationError("Missing REGISTRY_COORDINATOR_ADDRESS".into())
                    })?
                    .parse()
                    .map_err(|_| {
                        Error::ConfigurationError("Invalid REGISTRY_COORDINATOR_ADDRESS".into())
                    })?,
                operator_state_retriever_address: env::var("OPERATOR_STATE_RETRIEVER_ADDRESS")
                    .map_err(|_| {
                        Error::ConfigurationError("Missing OPERATOR_STATE_RETRIEVER_ADDRESS".into())
                    })?
                    .parse()
                    .map_err(|_| {
                        Error::ConfigurationError("Invalid OPERATOR_STATE_RETRIEVER_ADDRESS".into())
                    })?,
                delegation_manager_address: env::var("DELEGATION_MANAGER_ADDRESS")
                    .map_err(|_| {
                        Error::ConfigurationError("Missing DELEGATION_MANAGER_ADDRESS".into())
                    })?
                    .parse()
                    .map_err(|_| {
                        Error::ConfigurationError("Invalid DELEGATION_MANAGER_ADDRESS".into())
                    })?,
                service_manager_address: env::var("SERVICE_MANAGER_ADDRESS")
                    .map_err(|_| {
                        Error::ConfigurationError("Missing SERVICE_MANAGER_ADDRESS".into())
                    })?
                    .parse()
                    .map_err(|_| {
                        Error::ConfigurationError("Invalid SERVICE_MANAGER_ADDRESS".into())
                    })?,
                stake_registry_address: env::var("STAKE_REGISTRY_ADDRESS")
                    .map_err(|_| {
                        Error::ConfigurationError("Missing STAKE_REGISTRY_ADDRESS".into())
                    })?
                    .parse()
                    .map_err(|_| {
                        Error::ConfigurationError("Invalid STAKE_REGISTRY_ADDRESS".into())
                    })?,
                strategy_manager_address: env::var("STRATEGY_MANAGER_ADDRESS")
                    .map_err(|_| {
                        Error::ConfigurationError("Missing STRATEGY_MANAGER_ADDRESS".into())
                    })?
                    .parse()
                    .map_err(|_| {
                        Error::ConfigurationError("Invalid STRATEGY_MANAGER_ADDRESS".into())
                    })?,
                avs_directory_address: env::var("AVS_DIRECTORY_ADDRESS")
                    .map_err(|_| Error::ConfigurationError("Missing AVS_DIRECTORY_ADDRESS".into()))?
                    .parse()
                    .map_err(|_| {
                        Error::ConfigurationError("Invalid AVS_DIRECTORY_ADDRESS".into())
                    })?,
                rewards_coordinator_address: env::var("REWARDS_COORDINATOR_ADDRESS")
                    .map_err(|_| {
                        Error::ConfigurationError("Missing REWARDS_COORDINATOR_ADDRESS".into())
                    })?
                    .parse()
                    .map_err(|_| {
                        Error::ConfigurationError("Invalid REWARDS_COORDINATOR_ADDRESS".into())
                    })?,
                permission_controller_address: env::var("PERMISSION_CONTROLLER_ADDRESS")
                    .map_err(|_| {
                        Error::ConfigurationError("Missing PERMISSION_CONTROLLER_ADDRESS".into())
                    })?
                    .parse()
                    .map_err(|_| {
                        Error::ConfigurationError("Invalid PERMISSION_CONTROLLER_ADDRESS".into())
                    })?,
            };
            Ok(ProtocolSettings::from_eigenlayer(addresses))
        }
        Protocol::Tangle => {
            let settings = TangleInstanceSettings {
                blueprint_id: env::var("BLUEPRINT_ID")
                    .map_err(|_| Error::ConfigurationError("Missing BLUEPRINT_ID".into()))?
                    .parse()
                    .map_err(|_| Error::ConfigurationError("Invalid BLUEPRINT_ID".into()))?,
                service_id: env::var("SERVICE_ID")
                    .ok()
                    .map(|id| {
                        id.parse()
                            .map_err(|_| Error::ConfigurationError("Invalid SERVICE_ID".into()))
                    })
                    .transpose()?,
            };
            Ok(ProtocolSettings::from_tangle(settings))
        }
        _ => Err(Error::UnsupportedProtocol(protocol.to_string())),
    }
}

fn init_tracing_subscriber() {
    use tracing_subscriber::fmt::format::FmtSpan;
    use tracing_subscriber::prelude::*;

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_span_events(FmtSpan::CLOSE)
        .pretty();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(fmt_layer)
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
