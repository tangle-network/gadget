use color_eyre::Result;
use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "Gadget",
    about = "An MPC executor that connects to the Tangle network to perform work"
)]
struct Opt {
    /// Input file
    #[structopt(parse(from_os_str), short = "c", long = "config")]
    config: PathBuf,
    #[structopt(long, short = "v", parse(from_occurrences))]
    verbose: i32,
}

#[derive(Serialize, Deserialize)]
struct TomlConfig {
    bind_ip: String,
    bind_port: u16,
    bootnodes: Vec<String>,
    /// The genesis hash in hex format
    genesis_hash: String,
    /// The node key in hex format
    node_key: String,
    keystore_path: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let opt = Opt::from_args();
    setup_logger(opt.verbose, "gadget_shell")?;
    let config_contents = std::fs::read_to_string(opt.config)?;
    let config: TomlConfig = toml::from_str(&config_contents)?;
    /*
    // Hardcoded paths for now.
    let keystore_path = std::path::PathBuf::from(format!(
        "{}../../target/tangle/chains/local_testnet/keystore",
        env!("CARGO_MANIFEST_DIR")
    ));*/
    let keystore_path = std::path::PathBuf::from(config.keystore_path);

    let mut bootnodes = vec![];
    for bootnode in config.bootnodes.iter() {
        let addr: Multiaddr = bootnode.parse()?;
        bootnodes.push(addr);
    }

    let decoded_genesis_hash = hex::decode(config.genesis_hash)
        .map_err(|e| color_eyre::eyre::eyre!("Failed to parse genesis hash: {e}"))?;
    let decoded_node_key = hex::decode(config.node_key)
        .map_err(|e| color_eyre::eyre::eyre!("Failed to parse node key: {e}"))?;

    shell::run_forever(config::ShellConfig {
        keystore: config::KeystoreConfig::Path {
            path: keystore_path,
            password: None,
        },
        subxt: config::SubxtConfig {
            endpoint: url::Url::parse("ws://127.0.0.1:9944")?,
        },
        bind_ip: config.bind_ip,
        bind_port: config.bind_port,
        bootnodes,
        genesis_hash: <[u8; 32]>::try_from(decoded_genesis_hash.as_slice())
            .map_err(|e| color_eyre::eyre::eyre!("Failed to parse genesis hash: {e}"))?,
        node_key: <[u8; 32]>::try_from(decoded_node_key.as_slice())
            .map_err(|e| color_eyre::eyre::eyre!("Failed to parse node key: {e}"))?,
    })
    .await?;
    Ok(())
}

/// Sets up the logger for the shell, based on the verbosity level passed in.
///
/// Returns `Ok(())` on success, or `Err(anyhow::Error)` on failure.
///
/// # Arguments
///
/// * `verbosity` - An i32 integer representing the verbosity level.
/// * `filter` -  An &str representing filtering directive for EnvFilter
pub fn setup_logger(verbosity: i32, filter: &str) -> Result<()> {
    use tracing::Level;
    let log_level = match verbosity {
        0 => Level::ERROR,
        1 => Level::WARN,
        2 => Level::INFO,
        3 => Level::DEBUG,
        _ => Level::TRACE,
    };
    let env_filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive(format!("{filter}={log_level}").parse()?)
        .add_directive(format!("sync={log_level}").parse()?)
        .add_directive(format!("peerset={log_level}").parse()?)
        .add_directive(format!("sub-libp2p={log_level}").parse()?)
        .add_directive(format!("gadget={log_level}").parse()?);
    let logger = tracing_subscriber::fmt()
        .with_target(true)
        .with_max_level(log_level)
        .with_env_filter(env_filter);
    let logger = logger.pretty();

    logger.init();
    Ok(())
}

mod config;
mod keystore;
mod network;
mod protocols;
mod shell;
mod tangle;
