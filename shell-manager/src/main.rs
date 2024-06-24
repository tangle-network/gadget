use crate::protocols::resolver::{load_global_config_file, str_to_role_type, ProtocolMetadata};
use config::ShellManagerOpts;
use gadget_common::gadget_io;
use gadget_common::sp_core::Pair;
use gadget_io::tokio::io::AsyncWriteExt;
use gadget_io::ShellTomlConfig;
use shell_sdk::entry::keystore_from_base_path;
use shell_sdk::keystore::load_keys_from_keystore;
use shell_sdk::Client;
use shell_sdk::{entry, DebugLogger};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use structopt::StructOpt;
use tangle_environment::runtime::TangleRuntime;
use tangle_subxt::subxt;
use tangle_subxt::subxt::utils::AccountId32;

pub mod config;
pub mod error;
pub mod protocols;
pub mod utils;

#[gadget_io::tokio::main]
async fn main() -> color_eyre::Result<()> {
    //color_eyre::install()?;
    let opt = &ShellManagerOpts::from_args();
    let test_mode = opt.test;
    //    entry::setup_shell_logger(opt.verbose, opt.pretty, "gadget")
    //        .map_err(|err| msg_to_error(err.to_string()))?;
    entry::setup_shell_logger(opt.verbose, opt.pretty, "gadget")?;
    let shell_config_contents = std::fs::read_to_string(opt.shell_config.clone())?;
    let shell_config: ShellTomlConfig = toml::from_str(&shell_config_contents)
        .map_err(|err| utils::msg_to_error(err.to_string()))?;
    let global_protocols = load_global_config_file(opt.protocols_config.clone())
        .map_err(|err| utils::msg_to_error(err.to_string()))?;
    let logger = &DebugLogger {
        id: "Gadget Shell Manager".into(),
    };

    logger.info(format!(
        "Initializing with {} possible protocols",
        global_protocols.len()
    ));

    let subxt_client = subxt::OnlineClient::<subxt::PolkadotConfig>::from_url(&shell_config.url)
        .await
        .map_err(|err| utils::msg_to_error(err.to_string()))?;
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
        while let Some(notification) = runtime.next_event().await {
            logger.info(format!("Received notification {}", notification.number));
            let onchain_roles = utils::get_subscribed_role_types(
                &runtime,
                notification.hash,
                sub_account_id.clone(),
                &global_protocols,
                test_mode,
            )
            .await?;
            logger.trace(format!("OnChain roles: {onchain_roles:?}"));
            // Check to see if local does not have any running on-chain roles
            for role in &onchain_roles {
                let role_str = utils::get_role_type_str(role);
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
                            let host_os = utils::get_formatted_os_string();
                            let expected_hash = bin_hashes.get(&host_os).ok_or_else(|| {
                                utils::msg_to_error(format!("No hash for this OS ({host_os})"))
                            })?;

                            let sha_url = utils::get_sha_download_url(git, rev, package);

                            logger.info(format!("Downloading {role_str} SHA from {sha_url}"));
                            let sha_downloaded = reqwest::get(&sha_url)
                                .await
                                .map_err(|err| utils::msg_to_error(err.to_string()))?
                                .text()
                                .await
                                .map_err(|err| utils::msg_to_error(err.to_string()))?;

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

                            if utils::is_windows() {
                                binary_download_path += ".exe"
                            }

                            logger.info(format!("Downloading to {binary_download_path}"));

                            // Check if the binary exists, if not download it
                            let retrieved_hash =
                                if !utils::valid_file_exists(&binary_download_path, expected_hash)
                                    .await
                                {
                                    let url = utils::get_download_url(git, rev, package);

                                    let download = reqwest::get(&url)
                                        .await
                                        .map_err(|err| utils::msg_to_error(err.to_string()))?
                                        .bytes()
                                        .await
                                        .map_err(|err| utils::msg_to_error(err.to_string()))?;
                                    let retrieved_hash = utils::hash_bytes_to_hex(&download);

                                    // Write the binary to disk
                                    let mut file =
                                        gadget_io::tokio::fs::File::create(&binary_download_path)
                                            .await?;
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

                            if !utils::is_windows() {
                                if let Err(err) = utils::chmod_x_file(&binary_download_path).await {
                                    logger.warn(format!("Failed to chmod +x the binary: {err}"));
                                }
                            }

                            let arguments = utils::generate_process_arguments(&shell_config, opt)?;

                            logger.info(format!("Starting protocol: {role_str}"));

                            // Now that the file is loaded, spawn the process
                            let process_handle =
                                gadget_io::tokio::process::Command::new(&binary_download_path)
                                    .kill_on_drop(true)
                                    .stdout(std::process::Stdio::inherit()) // Inherit the stdout of this process
                                    .stderr(std::process::Stdio::inherit()) // Inherit the stderr of this process
                                    .stdin(std::process::Stdio::null())
                                    .current_dir(&std::env::current_dir()?)
                                    .envs(std::env::vars().collect::<Vec<_>>())
                                    .args(arguments)
                                    .spawn()?;

                            let (status_handle, abort) =
                                utils::generate_running_process_status_handle(
                                    process_handle,
                                    logger,
                                    &role_str,
                                );

                            active_shells.insert(role_str.clone(), (status_handle, Some(abort)));
                        }
                    }
                }
            }

            // Check to see if local is running protocols that are not on-chain
            let mut to_remove = vec![];
            for (role, process_handle) in &mut active_shells {
                let role_type = str_to_role_type(role)
                    .ok_or_else(|| utils::msg_to_error(format!("Invalid role type: {role}")))?;
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

        Err::<(), _>(utils::msg_to_error("Finality Notification stream died"))
    };

    let ctrlc_task = gadget_io::tokio::signal::ctrl_c();

    gadget_io::tokio::select! {
        res0 = manager_task => {
            Err(color_eyre::Report::msg(format!("Gadget Manager Closed Unexpectedly: {res0:?}")))
        },

        _ = ctrlc_task => {
            logger.info("CTRL-C detected, closing application");
            Ok(())
        }
    }
}
