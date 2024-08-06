use clap::{Parser, Subcommand};

mod create;

/// Gadget CLI tool
#[derive(Parser)]
#[clap(
    bin_name = "cargo-gadget",
    version,
    propagate_version = true,
    arg_required_else_help = true
)]
#[command(version = version())]
struct Cli {
    #[command(flatten)]
    manifest: clap_cargo::Manifest,
    #[command(flatten)]
    workspace: clap_cargo::Workspace,
    #[command(flatten)]
    features: clap_cargo::Features,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new blueprint
    Create {
        /// The name of the blueprint
        #[arg(short, long)]
        name: String,
    },
}

fn main() {
    // since this runs as a cargo subcommand, we need to skip the first argument
    // to get the actual arguments for the subcommand
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cli = Cli::parse_from(args);
    match cli.command {
        Commands::Create { name } => {
            println!("Generating blueprint with name: {}", name);
            create::new_blueprint(&name);
        }
    }
}

fn version() -> &'static str {
    option_env!("CARGO_VERSION_INFO").unwrap_or(env!("CARGO_PKG_VERSION"))
}
