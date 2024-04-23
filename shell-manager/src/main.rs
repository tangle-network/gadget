use crate::protocols::resolver::{load_global_config_file, str_to_role_type, ProtocolMetadata};
use gadget_common::sp_core::Pair;
use sha3::Digest;
use shell_sdk::config::defaults;
use shell_sdk::entry::keystore_from_base_path;
use shell_sdk::shell::load_keys_from_keystore;
use shell_sdk::tangle::TangleRuntime;
use shell_sdk::Client;
use shell_sdk::{entry, ClientWithApi, DebugLogger, ShellTomlConfig};
use std::collections::HashMap;
use std::path::PathBuf;
use structopt::StructOpt;
use tangle_subxt::subxt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

pub mod error;
pub mod protocols;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "Shell Manager",
    about = "An MPC executor that connects to the Tangle network and runs protocols dynamically on the fly"
)]
struct ShellManagerOpts {
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
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let opt = ShellManagerOpts::from_args();
    entry::setup_shell_logger(opt.verbose, opt.pretty, "gadget_shell")
        .map_err(|err| msg_to_error(err.to_string()))?;
    let shell_config_contents = std::fs::read_to_string(opt.shell_config)?;
    let shell_config: ShellTomlConfig =
        toml::from_str(&shell_config_contents).map_err(|err| msg_to_error(err.to_string()))?;
    let global_protocols = load_global_config_file(opt.protocols_config)
        .map_err(|err| msg_to_error(err.to_string()))?;
    let logger = DebugLogger {
        id: "Gadget Shell Manager".into(),
    };

    logger.info(format!(
        "Starting Gadget Shell Manager with {} possible protocols",
        global_protocols.len()
    ));

    let subxt_client = subxt::OnlineClient::<subxt::PolkadotConfig>::from_url(&shell_config.url)
        .await
        .map_err(|err| msg_to_error(err.to_string()))?;
    let runtime = TangleRuntime::new(subxt_client);

    let keystore = keystore_from_base_path(
        &shell_config.base_path,
        shell_config.chain,
        shell_config.keystore_password.clone(),
    );
    let (_role_key, acco_key) = load_keys_from_keystore(&keystore)?;
    let sub_account_id = subxt::utils::AccountId32(acco_key.public().0);

    let mut active_shells = HashMap::<String, RunningProtocolType>::new();

    while let Some(notification) = runtime.get_next_finality_notification().await {
        let onchain_roles = runtime
            .query_restaker_roles(notification.hash, sub_account_id.clone())
            .await
            .map_err(|err| msg_to_error(err.to_string()))?;
        // Check to see if local does not have any running on-chain roles
        'inner: for role in &onchain_roles {
            let role_str = format!("{role:?}");
            if !active_shells.contains_key(&role_str) {
                // Add in the protocol
                for global_protocol in &global_protocols {
                    if global_protocol.role_types().contains(&role.clone().into()) {
                        match global_protocol {
                            ProtocolMetadata::Internal { role_types } => {}

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
                                    let download = reqwest::get(&url)
                                        .await
                                        .map_err(|err| msg_to_error(err.to_string()))?
                                        .bytes()
                                        .await
                                        .map_err(|err| msg_to_error(err.to_string()))?;
                                    // The hash is sha3_256 of the binary
                                    let expected_hash = bin_hashes
                                        .get(&get_formatted_os_string())
                                        .ok_or_else(|| msg_to_error("No hash for this OS"))?;
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
                                    format!(
                                        "--bootnodes={}",
                                        shell_config
                                            .bootnodes
                                            .iter()
                                            .map(|r| r.to_string())
                                            .collect::<Vec<String>>()
                                            .join(",")
                                    ),
                                    format!(
                                        "--node-key={}",
                                        shell_config.node_key.clone().unwrap_or_else(|| {
                                            hex::encode(defaults::generate_node_key())
                                        })
                                    ),
                                    format!("--base-path={}", shell_config.base_path.display()),
                                    format!("--chain={}", shell_config.chain.to_string()),
                                    format!("--verbose={}", opt.verbose),
                                    format!("--pretty={}", opt.pretty),
                                ];

                                if let Some(keystore_password) = &shell_config.keystore_password {
                                    arguments
                                        .push(format!("--keystore-password={}", keystore_password));
                                }

                                logger.info(format!("Starting protocol: {role_str}"));

                                // Now that the file is loaded, spawn the shell
                                let process_handle =
                                    tokio::process::Command::new(&binary_download_path)
                                        .kill_on_drop(true)
                                        .stdout(std::process::Stdio::inherit()) // Inherit the stdout of this process
                                        .stderr(std::process::Stdio::inherit()) // Inherit the stderr of this process
                                        .args(arguments)
                                        .spawn()?;

                                active_shells.insert(
                                    role_str,
                                    RunningProtocolType::External(process_handle),
                                );
                            }
                        }
                        break 'inner;
                    }
                }
            }
        }

        // Check to see if local is running protocols that are not on-chain
        let mut to_remove = vec![];
        for (role, process_handle) in &mut active_shells {
            let role_type = str_to_role_type(role)
                .ok_or_else(|| msg_to_error(format!("Invalid role type: {role}")))?;
            if !onchain_roles.contains(&role_type) {
                logger.warn(format!("Killing protocol: {role}"));
                let _ = process_handle.kill().await;
                to_remove.push(role.clone());
            }
        }

        for role in to_remove {
            active_shells.remove(&role);
        }
    }

    Err(msg_to_error("Finality Notification stream died"))
}

enum RunningProtocolType {
    Internal(JoinHandle<()>),
    External(tokio::process::Child),
}

impl RunningProtocolType {
    async fn kill(&mut self) {
        match self {
            RunningProtocolType::Internal(handle) => {
                handle.abort();
            }
            RunningProtocolType::External(child) => {
                let _ = child.kill().await;
            }
        }
    }
}

async fn file_exists(path: &str) -> bool {
    tokio::fs::metadata(path).await.is_ok()
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

fn msg_to_error<T: Into<String>>(msg: T) -> color_eyre::Report {
    color_eyre::Report::msg(msg.into())
}
