use crate::protocols::resolver::{load_global_config_file, str_to_role_type, ProtocolMetadata};
use gadget_common::sp_core::Pair;
use sha2::Digest;
use shell_sdk::config::defaults;
use shell_sdk::entry::keystore_from_base_path;
use shell_sdk::shell::load_keys_from_keystore;
use shell_sdk::tangle::TangleRuntime;
use shell_sdk::Client;
use shell_sdk::{entry, ClientWithApi, DebugLogger, ShellTomlConfig};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use structopt::StructOpt;
use tangle_subxt::subxt;
use tangle_subxt::subxt::utils::AccountId32;
use tangle_subxt::tangle_testnet_runtime::api::jobs::events::job_refunded::RoleType;
use tokio::io::AsyncWriteExt;

pub mod error;
pub mod protocols;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "Shell Manager",
    about = "An MPC executor that connects to the Tangle network and runs protocols dynamically on the fly"
)]
struct ShellManagerOpts {
    /// The path to the shell configuration file
    #[structopt(parse(from_os_str), short = "s", long = "shell-config")]
    shell_config: PathBuf,
    #[structopt(parse(from_os_str), short = "p", long = "protocols-config")]
    protocols_config: PathBuf,
    /// The verbosity level, can be used multiple times
    #[structopt(long, short = "v", parse(from_occurrences))]
    verbose: i32,
    /// Whether to use pretty logging
    #[structopt(long)]
    pretty: bool,
    /// Wether this is debug mode or not
    #[structopt(long, short = "t")]
    test: bool,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let opt = &ShellManagerOpts::from_args();
    let test_mode = opt.test;
    //    entry::setup_shell_logger(opt.verbose, opt.pretty, "gadget")
    //        .map_err(|err| msg_to_error(err.to_string()))?;
    entry::setup_log();
    let shell_config_contents = std::fs::read_to_string(opt.shell_config.clone())?;
    let shell_config: ShellTomlConfig =
        toml::from_str(&shell_config_contents).map_err(|err| msg_to_error(err.to_string()))?;
    let global_protocols = load_global_config_file(opt.protocols_config.clone())
        .map_err(|err| msg_to_error(err.to_string()))?;
    let logger = &DebugLogger {
        id: "Gadget Shell Manager".into(),
    };

    logger.info(format!(
        "Initializing with {} possible protocols",
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
    let sub_account_id = AccountId32(acco_key.public().0);

    let mut active_shells = HashMap::<String, _>::new();

    let manager_task = async move {
        while let Some(notification) = runtime.get_next_finality_notification().await {
            println!("Received notification {}", notification.number);
            // TODO: Fetch blueprints instead of role types
            let onchain_roles = get_subscribed_role_types(
                &runtime,
                notification.hash,
                sub_account_id.clone(),
                &global_protocols,
                test_mode,
            )
            .await?;
            println!("OnChain roles: {onchain_roles:?}");
            // Check to see if local does not have any running on-chain roles
            'inner: for role in &onchain_roles {
                let role_str = get_role_type_str(role);
                if !active_shells.contains_key(&role_str) {
                    // Add in the protocol
                    for global_protocol in &global_protocols {
                        if global_protocol.role_types().contains(&role.clone()) {
                            let ProtocolMetadata {
                                role_types: _,
                                git,
                                rev,
                                package,
                                bin_hashes,
                            } = global_protocol;
                            // The hash is sha_256 of the binary
                            let host_os = get_formatted_os_string();
                            let expected_hash = bin_hashes.get(&host_os).ok_or_else(|| {
                                msg_to_error(format!("No hash for this OS ({host_os})"))
                            })?;

                            let sha_url = get_sha_download_url(git, rev, package);

                            logger.info(format!("Downloading {role_str} SHA from {sha_url}"));
                            let sha_downloaded = reqwest::get(&sha_url)
                                .await
                                .map_err(|err| msg_to_error(err.to_string()))?
                                .text()
                                .await
                                .map_err(|err| msg_to_error(err.to_string()))?;

                            if sha_downloaded.trim() != expected_hash.trim() {
                                logger.error(format!(
                                    "Retrieved hash {} mismatches the declared hash {} for protocol: {}",
                                    sha_downloaded,
                                    expected_hash,
                                    role_str
                                ));
                                continue;
                            }

                            let current_dir = std::env::current_dir()?;
                            let mut binary_download_path =
                                format!("{}/protocol-{rev}", current_dir.display());
                            if is_windows() {
                                binary_download_path += ".exe"
                            }

                            logger.info(format!("Downloading to {binary_download_path}"));

                            // Check if the binary exists, if not download it
                            let retrieved_hash =
                                if !valid_file_exists(&binary_download_path, expected_hash).await {
                                    let url = get_download_url(git, rev, package);

                                    let download = reqwest::get(&url)
                                        .await
                                        .map_err(|err| msg_to_error(err.to_string()))?
                                        .bytes()
                                        .await
                                        .map_err(|err| msg_to_error(err.to_string()))?;
                                    let retrieved_hash = hash_bytes_to_hex(&download);

                                    // Write the binary to disk
                                    let mut file =
                                        tokio::fs::File::create(&binary_download_path).await?;
                                    file.write_all(&download).await?;
                                    file.flush().await?;
                                    Some(retrieved_hash)
                                } else {
                                    None
                                };

                            if let Some(retrieved_hash) = retrieved_hash {
                                if retrieved_hash.trim() != expected_hash.trim() {
                                    logger.error(format!(
                                        "Binary hash {} mismatched expected hash of {} for protocol: {}",
                                        retrieved_hash,
                                        expected_hash,
                                        role_str
                                    ));
                                    continue;
                                }
                            }

                            chmod_x_file(&binary_download_path).await?;

                            let arguments = generate_process_arguments(&shell_config, opt)?;

                            logger.info(format!("Starting protocol: {role_str}"));

                            // Now that the file is loaded, spawn the process
                            let process_handle =
                                tokio::process::Command::new(&binary_download_path)
                                    .kill_on_drop(true)
                                    .stdout(std::process::Stdio::inherit()) // Inherit the stdout of this process
                                    .stderr(std::process::Stdio::inherit()) // Inherit the stderr of this process
                                    .stdin(std::process::Stdio::null())
                                    .current_dir(&std::env::current_dir()?)
                                    .envs(std::env::vars().collect::<Vec<_>>())
                                    .args(arguments)
                                    .spawn()?;

                            let (status_handle, abort) = generate_running_process_status_handle(
                                process_handle,
                                logger,
                                &role_str,
                            );

                            active_shells.insert(role_str, (status_handle, Some(abort)));

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
                    if let Some(abort_handle) = process_handle.1.take() {
                        let _ = abort_handle.send(());
                    }

                    to_remove.push(role.clone());
                }
            }

            // Check to see if any process handles have died
            for (role, process_handle) in &mut active_shells {
                if !process_handle.0.load(Ordering::Relaxed) {
                    // By removing any killed processes, we will auto-restart them on the next finality notification if required
                    to_remove.push(role.clone());
                }
            }

            for role in to_remove {
                active_shells.remove(&role);
            }
        }

        Err::<(), _>(msg_to_error("Finality Notification stream died"))
    };

    let ctrlc_task = tokio::signal::ctrl_c();

    tokio::select! {
        res0 = manager_task => {
            Err(color_eyre::Report::msg(format!("Gadget Manager Closed Unexpectedly: {res0:?}")))
        },

        _ = ctrlc_task => {
            logger.info("CTRL-C detected, closing application");
            Ok(())
        }
    }
}

async fn get_subscribed_role_types(
    runtime: &TangleRuntime,
    block_hash: [u8; 32],
    account_id: AccountId32,
    global_protocols: &[ProtocolMetadata],
    test_mode: bool,
) -> color_eyre::Result<Vec<RoleType>> {
    if test_mode {
        return Ok(global_protocols
            .iter()
            .flat_map(|r| r.role_types().clone())
            .collect());
    }

    runtime
        .query_restaker_roles(block_hash, account_id)
        .await
        .map_err(|err| msg_to_error(err.to_string()))
}

fn generate_process_arguments(
    shell_config: &ShellTomlConfig,
    opt: &ShellManagerOpts,
) -> color_eyre::Result<Vec<String>> {
    let mut arguments = vec![
        format!("--bind-ip={}", shell_config.bind_ip),
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
            shell_config
                .node_key
                .clone()
                .unwrap_or_else(|| { hex::encode(defaults::generate_node_key()) })
        ),
        format!("--base-path={}", shell_config.base_path.display()),
        format!("--chain={}", shell_config.chain.to_string()),
        format!("--verbose={}", opt.verbose),
        format!("--pretty={}", opt.pretty),
    ];

    if let Some(keystore_password) = &shell_config.keystore_password {
        arguments.push(format!("--keystore-password={}", keystore_password));
    }

    Ok(arguments)
}

fn hash_bytes_to_hex<T: AsRef<[u8]>>(input: T) -> String {
    let mut hasher = sha2::Sha256::default();
    hasher.update(input);
    hex::encode(hasher.finalize())
}

async fn valid_file_exists(path: &str, expected_hash: &str) -> bool {
    // The hash is sha3_256 of the binary
    if let Ok(file) = tokio::fs::read(path).await {
        // Compute the SHA3-256
        let retrieved_bytes = hash_bytes_to_hex(file);
        expected_hash == retrieved_bytes.as_str()
    } else {
        false
    }
}

fn get_formatted_os_string() -> String {
    let os = std::env::consts::OS;

    match os {
        "macos" => "apple-darwin".to_string(),
        "windows" => "pc-windows-msvc".to_string(),
        "linux" => "unknown-linux-gnu".to_string(),
        _ => os.to_string(),
    }
}

fn get_download_url<T: Into<String>>(git: T, rev: &str, package: &str) -> String {
    let os = get_formatted_os_string();
    let arch = std::env::consts::ARCH;

    let mut git = git.into();

    // Ensure the first part of the url ends with `/`
    if git.ends_with(".git") {
        git = git.replace(".git", "/")
    }

    if !git.ends_with('/') {
        git.push('/')
    }

    let ext = if os == "windows" { ".exe" } else { "" };

    // https://github.com/webb-tools/protocols/releases/download/protocol-aarch64-apple-darwin-d1cc78ee4652696a2228f62b735e55ef23bf3f8e/protocol-threshold-bls-protocol-d1cc78ee4652696a2228f62b735e55ef23bf3f8e.sha256
    // https://github.com/webb-tools/protocols/releases/download/aarch64-apple-darwin-d1cc78ee4652696a2228f62b735e55ef23bf3f8e/protocol-threshold-bls-protocol-d1cc78ee4652696a2228f62b735e55ef23bf3f8e.sha256
    format!("{git}releases/download/{arch}-{os}-{rev}/protocol-{package}-{rev}{ext}")
}

fn get_sha_download_url<T: Into<String>>(git: T, rev: &str, package: &str) -> String {
    let base_url = get_download_url(git, rev, package);
    format!("{base_url}.sha256")
}

fn msg_to_error<T: Into<String>>(msg: T) -> color_eyre::Report {
    color_eyre::Report::msg(msg.into())
}

fn get_role_type_str(role_type: &RoleType) -> String {
    match role_type {
        RoleType::Tss(tss) => format!("{tss:?}"),
        RoleType::ZkSaaS(zksaas) => format!("{zksaas:?}"),
        RoleType::LightClientRelaying => format!("{role_type:?}"),
    }
}

async fn chmod_x_file<P: AsRef<Path>>(path: P) -> color_eyre::Result<()> {
    let success = tokio::process::Command::new("chmod")
        .arg("+x")
        .arg(format!("{}", path.as_ref().display()))
        .spawn()?
        .wait_with_output()
        .await?
        .status
        .success();

    if success {
        Ok(())
    } else {
        Err(color_eyre::eyre::eyre!(
            "Failed to chmod +x {}",
            path.as_ref().display()
        ))
    }
}

fn is_windows() -> bool {
    std::env::consts::OS == "windows"
}

fn generate_running_process_status_handle(
    process: tokio::process::Child,
    logger: &DebugLogger,
    role_type: &str,
) -> (Arc<AtomicBool>, tokio::sync::oneshot::Sender<()>) {
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    let status = Arc::new(AtomicBool::new(true));
    let status_clone = status.clone();
    let logger = logger.clone();
    let role_type = role_type.to_string();

    let task = async move {
        let output = process.wait_with_output().await;
        logger.warn(format!("Process for {role_type} exited: {output:?}"));
        status_clone.store(false, Ordering::Relaxed);
    };

    let task = async move {
        tokio::select! {
            _ = stop_rx => {},
            _ = task => {},
        }
    };

    tokio::spawn(task);
    (status, stop_tx)
}
