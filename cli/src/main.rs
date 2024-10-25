use std::path::PathBuf;

use cargo_tangle::{create, deploy, keys};
use clap::{Parser, Subcommand};
use keys::KeyType;

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
        subcommand: GadgetCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum GadgetCommands {
    /// Create a new blueprint
    #[command(visible_alias = "c")]
    Create {
        /// The name of the blueprint
        #[arg(short, long)]
        name: String,

        #[command(flatten)]
        source: Option<create::Source>,
    },

    /// Deploy a blueprint to the Tangle Network.
    #[command(visible_alias = "d")]
    Deploy {
        /// HTTP RPC URL to use
        #[arg(long, value_name = "URL", default_value = "https://rpc.tangle.tools")]
        http_rpc_url: String,
        /// Tangle RPC URL to use
        #[arg(long, value_name = "URL", default_value = "wss://rpc.tangle.tools")]
        ws_rpc_url: String,
        /// The package to deploy (if the workspace has multiple packages).
        #[arg(short, long, value_name = "PACKAGE")]
        package: Option<String>,
    },
    /// Generate a key
    Keygen {
        /// The type of key to generate
        #[arg(short, long, value_enum)]
        key_type: KeyType,

        /// The path to save the key (optional)
        #[arg(short, long)]
        path: Option<PathBuf>,

        /// The SURI or seed to use for the generation of the key (optional)
        #[arg(short, long)]
        seed: Option<String>,

        /// If true, the secret key will be printed along with the public key
        #[arg(long)]
        show_secret: bool,
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
        Commands::Blueprint { subcommand } => match subcommand {
            GadgetCommands::Create { name, source } => {
                create::new_blueprint(name, source);
            }
            GadgetCommands::Deploy {
                http_rpc_url,
                ws_rpc_url,
                package,
            } => {
                let manifest_path = cli
                    .manifest
                    .manifest_path
                    .unwrap_or_else(|| PathBuf::from("Cargo.toml"));
                let _ = deploy::deploy_to_tangle(deploy::Opts {
                    http_rpc_url,
                    ws_rpc_url,
                    manifest_path,
                    pkg_name: package,
                    signer: None,
                    signer_evm: None,
                })
                .await?;
            }
            GadgetCommands::Keygen {
                key_type,
                path,
                seed,
                show_secret,
            } => {
                keys::generate_key(
                    key_type,
                    path,
                    seed.as_deref().map(str::as_bytes),
                    show_secret,
                )?;
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
