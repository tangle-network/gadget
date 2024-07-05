use crate::protocols::resolver::{load_global_config_file, ProtocolMetadata};
use config::ShellManagerOpts;
use gadget_common::gadget_io;
use gadget_common::prelude::GadgetEnvironment;
use gadget_common::sp_core::Pair;
use gadget_io::tokio::io::AsyncWriteExt;
use gadget_io::ShellTomlConfig;
use shell_sdk::entry::keystore_from_base_path;
use shell_sdk::keystore::load_keys_from_keystore;
use shell_sdk::{entry, Client, DebugLogger};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use structopt::StructOpt;
use tangle_environment::api::ServicesClient;
use tangle_environment::gadget::SubxtConfig;
use tangle_environment::TangleEnvironment;
use tangle_subxt::subxt::utils::AccountId32;

pub mod config;
pub mod error;
pub mod protocols;
pub mod utils;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    //color_eyre::install()?;
    let opt = &ShellManagerOpts::from_args();
    let test_mode = opt.test;
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

    let keystore = keystore_from_base_path(
        &shell_config.base_path,
        shell_config.chain,
        shell_config.keystore_password.clone(),
    );

    let (_role_key, acco_key) = load_keys_from_keystore(&keystore)?;
    let sub_account_id = AccountId32(acco_key.public().0);
    let subxt_config = SubxtConfig {
        endpoint: shell_config.url.clone(),
    };

    let tangle_environment = TangleEnvironment::new(subxt_config, acco_key, logger.clone());

    let tangle_runtime = tangle_environment.setup_runtime().await?;
    let runtime = ServicesClient::new(logger.clone(), tangle_runtime.client());

    let mut active_shells = HashMap::<String, _>::new();

    let manager_task = async move {
        while let Some(event) = tangle_runtime.next_event().await {
            logger.info(format!("Received notification {}", event.number));
            let onchain_services = utils::get_subscribed_services(
                &runtime,
                event.hash,
                sub_account_id.clone(),
                &global_protocols,
                test_mode,
            )
            .await?;
            logger.trace(format!(
                "OnChain services: {:?}",
                onchain_services
                    .iter()
                    .map(|r| r.metadata.name.clone())
                    .collect::<Vec<_>>()
            ));
            // Check to see if local does not have any running on-chain roles
            for role in &onchain_services {
                let service_str = utils::get_service_str(role);
                if !active_shells.contains_key(&service_str) {
                    // Add in the protocol
                    for global_protocol in &global_protocols {
                        if global_protocol.service.metadata.name == role.metadata.name {
                            let ProtocolMetadata {
                                service: _,
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

                            logger.info(format!("Downloading {service_str} SHA from {sha_url}"));
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
                                    service_str
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
                                        service_str
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

                            logger.info(format!("Starting protocol: {service_str}"));

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
                                    &service_str,
                                );

                            active_shells.insert(service_str.clone(), (status_handle, Some(abort)));
                        }
                    }
                }
            }

            // Check to see if local is running protocols that are not on-chain
            let mut to_remove = vec![];
            for (role, process_handle) in &mut active_shells {
                for onchain_service in &onchain_services {
                    let onchain_service_str = utils::get_service_str(onchain_service);
                    if &onchain_service_str != role {
                        logger.warn(format!(
                            "Killing service that is no longer on-chain: {role}"
                        ));
                        if let Some(abort_handle) = process_handle.1.take() {
                            let _ = abort_handle.send(());
                        }

                        to_remove.push(role.clone());
                    }
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
