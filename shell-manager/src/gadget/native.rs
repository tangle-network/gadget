use crate::config::ShellManagerOpts;
use crate::gadget::ActiveShells;
use crate::protocols::resolver::ProtocolMetadata;
use crate::utils;
use gadget_common::prelude::DebugLogger;
use gadget_io::ShellTomlConfig;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::{
    Gadget, GadgetSourceFetcher, GithubFetcher, ServiceBlueprint,
};
use tokio::io::AsyncWriteExt;

pub async fn handle(
    onchain_services: &[ServiceBlueprint],
    shell_config: &ShellTomlConfig,
    shell_manager_opts: &ShellManagerOpts,
    active_shells: &mut ActiveShells,
    global_protocols: &[ProtocolMetadata],
    logger: &DebugLogger,
) -> color_eyre::Result<()> {
    for service in onchain_services {
        if let Gadget::Native(gadget) = &service.gadget {
            let source = &gadget.soruces[0];
            if let GadgetSourceFetcher::Github(gh) = source {
                if let Err(err) = handle_github_source(
                    service,
                    shell_config,
                    shell_manager_opts,
                    gh,
                    active_shells,
                    global_protocols,
                    logger,
                )
                .await
                {
                    logger.warn(err)
                }
            } else {
                logger.warn(format!("The source {source:?} is not supported",))
            }
        }
    }

    Ok(())
}

async fn handle_github_source(
    service: &ServiceBlueprint,
    shell_config: &ShellTomlConfig,
    shell_manager_opts: &ShellManagerOpts,
    github: &GithubFetcher,
    active_shells: &mut ActiveShells,
    global_protocols: &[ProtocolMetadata],
    logger: &DebugLogger,
) -> color_eyre::Result<()> {
    let service_str = utils::get_service_str(service);
    if !active_shells.contains_key(&service_str) {
        // Add in the protocol
        for global_protocol in global_protocols {
            if global_protocol.service.metadata.name == service.metadata.name {
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
                        sha_downloaded, expected_hash, service_str
                    ));
                    continue;
                }

                let current_dir = std::env::current_dir()?;
                let mut binary_download_path = format!("{}/protocol-{rev}", current_dir.display());

                if utils::is_windows() {
                    binary_download_path += ".exe"
                }

                logger.info(format!("Downloading to {binary_download_path}"));

                // Check if the binary exists, if not download it
                let retrieved_hash =
                    if !utils::valid_file_exists(&binary_download_path, expected_hash).await {
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
                            gadget_io::tokio::fs::File::create(&binary_download_path).await?;
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
                            retrieved_hash, expected_hash, service_str
                        ));
                        continue;
                    }
                }

                if !utils::is_windows() {
                    if let Err(err) = utils::chmod_x_file(&binary_download_path).await {
                        logger.warn(format!("Failed to chmod +x the binary: {err}"));
                    }
                }

                let arguments =
                    utils::generate_process_arguments(shell_config, shell_manager_opts)?;

                logger.info(format!("Starting protocol: {service_str}"));

                // Now that the file is loaded, spawn the process
                let process_handle = gadget_io::tokio::process::Command::new(&binary_download_path)
                    .kill_on_drop(true)
                    .stdout(std::process::Stdio::inherit()) // Inherit the stdout of this process
                    .stderr(std::process::Stdio::inherit()) // Inherit the stderr of this process
                    .stdin(std::process::Stdio::null())
                    .current_dir(&std::env::current_dir()?)
                    .envs(std::env::vars().collect::<Vec<_>>())
                    .args(arguments)
                    .spawn()?;

                let (status_handle, abort) = utils::generate_running_process_status_handle(
                    process_handle,
                    logger,
                    &service_str,
                );

                active_shells.insert(service_str.clone(), (status_handle, Some(abort)));
            }
        }
    }

    Ok(())
}

fn sha_hex_string_to_slice_32<T: AsRef<str>>(hash: T) -> Result<[u8; 32], &'static str> {
    let hash = hash.as_ref();
    if hash.len() != 64 {
        return Err("Hash must be exactly 64 characters long");
    }

    let mut bytes = [0u8; 32];
    for (i, byte_chunk) in hash.as_bytes().chunks(2).enumerate() {
        let hex_str = std::str::from_utf8(byte_chunk).map_err(|_| "Invalid UTF-8 sequence")?;
        bytes[i] = u8::from_str_radix(hex_str, 16).map_err(|_| "Failed to parse hex string")?;
    }

    Ok(bytes)
}
