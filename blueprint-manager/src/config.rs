use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "Blueprint Manager",
    about = "An program executor that connects to the Tangle network and runs protocols dynamically on the fly"
)]
pub struct BlueprintManagerConfig {
    /// The path to the gadget configuration file
    #[arg(short = 's', long)]
    pub gadget_config: Option<PathBuf>,
    /// The path to the keystore
    #[arg(short = 'k', long)]
    pub keystore_uri: String,
    /// The directory in which all gadgets will store their data
    #[arg(long, short = 'd', default_value = "./data")]
    pub data_dir: PathBuf,
    /// The verbosity level, can be used multiple times to increase verbosity
    #[arg(long, short = 'v', value_parser = clap::value_parser!(u8).range(0..=255))]
    pub verbose: u8,
    /// Whether to use pretty logging
    #[arg(long)]
    pub pretty: bool,
    /// An optional unique string identifier for the blueprint manager to differentiate between multiple
    /// running instances of a BlueprintManager (mostly for debugging purposes)
    #[arg(long, alias = "id")]
    pub instance_id: Option<String>,
    #[arg(long, short = 't')]
    pub test_mode: bool,
}
