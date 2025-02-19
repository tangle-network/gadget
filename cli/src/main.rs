use std::path::PathBuf;

use crate::deploy::tangle::{deploy_to_tangle, Opts};
use cargo_tangle::create::BlueprintType;
#[cfg(feature = "eigenlayer")]
use cargo_tangle::deploy::eigenlayer::{deploy_to_eigenlayer, EigenlayerDeployOpts};
use cargo_tangle::run::eigenlayer::{run_eigenlayer_avs, RunOpts};
use cargo_tangle::{create, deploy, keys};
use clap::{Parser, Subcommand};
use gadget_crypto::KeyTypeId;

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
        #[arg(short, long, value_enum)]
        key_type: KeyTypeId,
        /// The path to save the key to
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// The seed to use for key generation (hex format without 0x prefix)
        #[arg(long)]
        seed: Option<Vec<u8>>,
        /// Show the secret key in output
        #[arg(short, long)]
        show_secret: bool,
    },
    /// Import a key into the keystore
    #[command(visible_alias = "i")]
    Import {
        /// The type of key to import (sr25519, ed25519, ecdsa, bls381, bls377, bn254)
        #[arg(short, long, value_enum)]
        key_type: KeyTypeId,
        /// The secret key to import (hex format without 0x prefix)
        #[arg(short, long)]
        secret: String,
        /// The path to the keystore
        #[arg(short, long)]
        keystore_path: PathBuf,
    },
    /// Export a key from the keystore
    #[command(visible_alias = "e")]
    Export {
        /// The type of key to export (sr25519, ed25519, ecdsa, bls381, bls377, bn254)
        #[arg(short, long, value_enum)]
        key_type: KeyTypeId,
        /// The public key to export (hex format without 0x prefix)
        #[arg(short, long)]
        public: String,
        /// The path to the keystore
        #[arg(short, long)]
        keystore_path: PathBuf,
    },
    /// List all keys in the keystore
    #[command(visible_alias = "l")]
    List {
        /// The path to the keystore
        #[arg(short, long)]
        keystore_path: PathBuf,
    },
    /// Generate a new mnemonic phrase
    #[command(visible_alias = "m")]
    GenerateMnemonic {
        /// Number of words in the mnemonic (12, 15, 18, 21, or 24)
        #[arg(short, long, value_parser = clap::value_parser!(u32).range(12..=24))]
        word_count: Option<u32>,
    },
}

#[derive(Subcommand, Debug)]
pub enum BlueprintCommands {
    /// Create a new blueprint
    #[command(visible_alias = "c")]
    Create {
        /// The name of the blueprint
        #[arg(short, long, value_name = "NAME", env = "NAME")]
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

    /// Run an Eigenlayer AVS
    #[command(visible_alias = "r")]
    Run {
        /// HTTP RPC endpoint for the Ethereum network
        #[arg(long, value_name = "URL", env = "ETH_RPC_URL")]
        http_rpc_url: String,

        /// The private key for the aggregator (in hex format without 0x prefix)
        #[arg(long, value_name = "KEY", env = "AGGREGATOR_KEY")]
        aggregator_key: String,

        /// The port to run the aggregator service on
        #[arg(long, value_name = "PORT", default_value = "8081")]
        port: u16,

        /// The task manager contract address
        #[arg(long, value_name = "ADDRESS")]
        task_manager: String,

        /// Path to the blueprint directory containing the AVS code
        #[arg(long, value_name = "PATH")]
        blueprint_path: PathBuf,
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
        #[arg(short, long, value_name = "PACKAGE", env = "CARGO_PACKAGE")]
        package: Option<String>,
    },
    /// Deploy to Eigenlayer
    #[cfg(feature = "eigenlayer")]
    Eigenlayer {
        /// HTTP RPC URL to use
        #[arg(long, value_name = "URL", env)]
        rpc_url: String,
        /// Path to the contracts
        #[arg(long)]
        contracts_path: Option<String>,
        /// Whether to deploy contracts in an interactive ordered manner
        #[arg(long)]
        ordered_deployment: bool,
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
                } => {
                    deploy_to_eigenlayer(EigenlayerDeployOpts::new(
                        rpc_url,
                        contracts_path,
                        ordered_deployment,
                    ))
                    .await?;
                }
            },
            BlueprintCommands::Run {
                http_rpc_url,
                aggregator_key,
                port,
                task_manager,
                blueprint_path,
            } => {
                run_eigenlayer_avs(RunOpts {
                    http_rpc_url,
                    aggregator_key,
                    port,
                    task_manager,
                    blueprint_path,
                })
                .await?;
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
            } => {
                let public = keys::import_key(key_type, &secret, &keystore_path)?;
                eprintln!("Imported {:?} key:", key_type);
                eprintln!("Public key: {}", public);
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
