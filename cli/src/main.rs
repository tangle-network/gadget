use clap::{Parser, Subcommand};

mod generate;

/// Gadget CLI tool
#[derive(Parser)]
#[command(version, about, long_about = None)]
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
    /// Generate a new blueprint
    Generate {
        /// The name of the blueprint
        #[arg(short, long)]
        name: String,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Generate { name } => {
            println!("Generating blueprint with name: {}", name);
            generate::generate_blueprint(&name);
        }
    }
}
