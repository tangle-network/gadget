use crate::protocols::config::ProtocolConfig;
use crate::protocols::resolver::{load_global_config_file, ProtocolMetadata};
use color_eyre::Result;
use sha3::Digest;
use shell_sdk::config::defaults;
use shell_sdk::{DebugLogger, ShellTomlConfig};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use structopt::StructOpt;
use tangle_subxt::subxt;
use tangle_subxt::tangle_mainnet_runtime::api::jobs::events::job_refunded::RoleType;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

pub mod error;
pub mod protocols;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "Shell Manager",
    about = "An MPC executor that connects to the Tangle network and runs protocols dynamically on the fly"
)]
struct Opt {
    /// The path to the shell configuration file
    #[structopt(global = true, parse(from_os_str), short = "s", long = "shell-config")]
    shell_config: PathBuf,
    #[structopt(
        global = true,
        parse(from_os_str),
        short = "p",
        long = "protocols-config"
    )]
    protocols_config: PathBuf,
    /// The verbosity level, can be used multiple times
    #[structopt(long, short = "v", global = true, parse(from_occurrences))]
    verbose: i32,
    /// Whether to use pretty logging
    #[structopt(global = true, long)]
    pretty: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let opt = Opt::from_args();
    setup_logger(&opt, "gadget_shell")?;
    let shell_config_contents = std::fs::read_to_string(opt.shell_config)?;
    let shell_config: ShellTomlConfig = toml::from_str(&shell_config_contents)?;
    let global_protocols = load_global_config_file(opt.protocols_config)?;
    let logger = DebugLogger {
        id: "Gadget Shell Manager".into(),
    };

    logger.info(format!(
        "Starting Gadget Shell Manager with {} possible protocols",
        global_protocols.len()
    ));

    let mut active_shells = Arc::new(Mutex::new(HashMap::<Vec<RoleType>, _>::new()));

    loop {
        let onchain_roles = get_roles().await;
        let mut active_shells = active_shells.lock().await;
        'inner: for role in onchain_roles {
            if !active_shells.contains_key(&role) {
                // Add in the protocol
                for global_protocol in &global_protocols {
                    if global_protocol.role_types() == &role {
                        match global_protocol {
                            ProtocolMetadata::Internal { .. } => {}
                            ProtocolMetadata::External {
                                git,
                                rev,
                                bin_hashes,
                                ..
                            } => {
                                let binary_download_path = format!("protocol-{rev}");

                                // Check if the binary exists, if not download it
                                if !file_exists(&binary_download_path).await {
                                    let url = get_download_url(git, rev);
                                    let download = reqwest::get(&url).await?.bytes().await?;
                                    // The hash is sha3_256 of the binary
                                    let expected_hash = bin_hashes
                                        .get(&get_formatted_os_string())
                                        .ok_or("No hash for this OS")?;
                                    let mut hasher = sha3::Sha3_256::default();
                                    hasher.update(&download);
                                    let retrieved_hash = hex::encode(hasher.finalize());
                                    if retrieved_hash != *expected_hash {
                                        logger.error(format!(
                                            "Binary hash mismatch for protocol: {}",
                                            retrieved_hash
                                        ));
                                        continue;
                                    }

                                    // Write the binary to disk
                                    let mut file =
                                        tokio::fs::File::create(&binary_download_path).await?;
                                    file.write_all(&download).await?;
                                }

                                let open_port =
                                    TcpListener::bind(format!("{}:0", shell_config.bind_ip))
                                        .await?
                                        .local_addr()?
                                        .port();

                                let mut arguments = vec![
                                    format!("--bind-ip={}", shell_config.bind_ip),
                                    format!("--bind-port={open_port}"),
                                    format!("--url={}", shell_config.url),
                                    format!("--bootnodes={}", shell_config.bootnodes.join(",")),
                                    format!(
                                        "--node-key={}",
                                        shell_config.node_key.clone().unwrap_or_else(|| {
                                            hex::encode(defaults::generate_node_key())
                                        })
                                    ),
                                    format!("--base-path={}", shell_config.base_path.display()),
                                    format!("--chain={}", shell_config.chain.to_string()),
                                ];

                                if let Some(keystore_password) = &shell_config.keystore_password {
                                    arguments
                                        .push(format!("--keystore-password={}", keystore_password));
                                }

                                // Now that the file is loaded, spawn the shell
                                let process_handle =
                                    tokio::process::Command::new(&binary_download_path)
                                        .kill_on_drop(true)
                                        .args(arguments)
                                        .spawn()?;

                                active_shells.insert(role, process_handle);
                            }
                        }
                        break 'inner;
                    }
                }
            }
        }

        // Sleep for 1 block (6 seconds) before polling again for new potential shells to spawn/manage
        tokio::time::sleep(Duration::from_secs(6));
    }
}

async fn file_exists(path: &str) -> bool {
    tokio::fs::metadata(path).await.is_ok()
}

async fn get_roles() -> Vec<Vec<RoleType>> {
    unimplemented!()
}

fn get_formatted_os_string() -> String {
    let os = std::env::consts::OS;

    match os {
        "macos" => "apple-darwin".to_string(),
        "windows" => "windows".to_string(),
        "linux" => "unknown-linux-gnu".to_string(),
        _ => os.to_string(),
    }
}

fn get_download_url<T: Into<String>>(git: T, rev: &str) -> String {
    let os = get_formatted_os_string();
    let arch = std::env::consts::ARCH;

    let mut git = git.into();

    // Ensure the first part of the url ends with `/`
    if git.ends_with(".git") {
        git = git.replace(".git", "/")
    } else {
        if !git.ends_with("/") {
            git.push_str("/")
        }
    }

    // https://github.com/webb-tools/protocol-template/releases/download/protocol-x86_64-apple-darwin/protocol-6fa01cb5cf684e2d0272252f00ea81d6f269f3d7
    format!("{git}releases/download/protocol-{arch}-{os}/protocol-{rev}")
}

/*
#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let opt = Opt::from_args();
    setup_logger(&opt, "gadget_shell")?;
    let config: shell_sdk::config::TomlConfig = if let Some(config) = opt.config {
        let config_contents = std::fs::read_to_string(config)?;
        toml::from_str(&config_contents)?
    } else {
        opt.options
    };

    // Setup the client to allow polling the chain for active roles
    let subxt_client =
        subxt::OnlineClient::<subxt::PolkadotConfig>::from_url(&config.url).await?;

    let mut active_shells = HashMap::new();

    // Periodically (every block) poll the chain for active roles, see if we need to run any, and run them
    loop {
        subxt_client.runtime_api().at_latest().await?.
        tokio::time::sleep(Duration::from_secs(6));
    }

    //
    let open_port = TcpListener::bind("0.0.0.0:0").await?.local_addr()?.port();

    shell_sdk::run_forever(shell_sdk::ShellConfig {
        keystore: shell_sdk::KeystoreConfig::Path {
            path: config
                .base_path
                .join("chains")
                .join(config.chain.to_string())
                .join("keystore"),
            password: config.keystore_password.map(|s| s.into()),
        },
        subxt: shell_sdk::SubxtConfig {
            endpoint: config.url,
        },
        base_path: config.base_path,
        bind_ip: config.bind_ip,
        bind_port: open_port,
        bootnodes: config.bootnodes,
        node_key: hex::decode(
            config
                .node_key
                .unwrap_or_else(|| hex::encode(defaults::generate_node_key())),
        )?
        .try_into()
        .map_err(|_| {
            color_eyre::eyre::eyre!("Invalid node key length, expect 32 bytes hex string")
        })?,
    })
    .await?;
    Ok(())
}
*/
/// Sets up the logger for the shell-sdk, based on the verbosity level passed in.
fn setup_logger(opt: &Opt, filter: &str) -> Result<()> {
    use tracing::Level;
    let log_level = match opt.verbose {
        0 => Level::ERROR,
        1 => Level::WARN,
        2 => Level::INFO,
        3 => Level::DEBUG,
        _ => Level::TRACE,
    };
    let env_filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive(format!("{filter}={log_level}").parse()?)
        .add_directive(format!("gadget={log_level}").parse()?);
    let logger = tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .with_line_number(false)
        .without_time()
        .with_max_level(log_level)
        .with_env_filter(env_filter);
    if opt.pretty {
        logger.pretty().init();
    } else {
        logger.compact().init();
    }
    Ok(())
}
