use std::path::PathBuf;

use cargo_tangle::{create, deploy};
use clap::{Parser, Subcommand};

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
enum GadgetCommands {
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
        /// Tangle RPC URL to use
        #[arg(long, value_name = "URL", default_value = "wss://rpc.tangle.tools")]
        rpc_url: String,
        /// The package to deploy (if the workspace has multiple packages).
        #[arg(short, long, value_name = "PACKAGE")]
        package: Option<String>,
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
            GadgetCommands::Deploy { rpc_url, package } => {
                let manifest_path = cli
                    .manifest
                    .manifest_path
                    .unwrap_or_else(|| PathBuf::from("Cargo.toml"));
                let _ = deploy::deploy_to_tangle(deploy::Opts {
                    rpc_url,
                    manifest_path,
                    pkg_name: package,
                    signer: None,
                    signer_evm: None,
                })
                .await?;
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
