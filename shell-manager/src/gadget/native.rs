use crate::config::ShellManagerOpts;
use crate::gadget::ActiveShells;
use crate::protocols::resolver::NativeGithubMetadata;
use crate::utils;
use crate::utils::{bytes_to_utf8_string, get_service_str};
use color_eyre::eyre::OptionExt;
use gadget_common::prelude::DebugLogger;
use gadget_io::ShellTomlConfig;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::{
    GadgetBinary, GadgetSourceFetcher, GithubFetcher, ServiceBlueprint,
};
use tokio::io::AsyncWriteExt;

pub async fn handle(
    onchain_services: &[NativeGithubMetadata],
    onchain_gh_fetchers: &[&GithubFetcher],
    shell_config: &ShellTomlConfig,
    shell_manager_opts: &ShellManagerOpts,
    active_shells: &mut ActiveShells,
    global_protocols: &[NativeGithubMetadata],
    logger: &DebugLogger,
) -> color_eyre::Result<()> {
    for (gh, fetcher) in onchain_services.into_iter().zip(onchain_gh_fetchers) {
        let bin_hashes = fetcher
            .binaries
            .0
            .iter()
            .map(|r| slice_32_to_sha_hex_string(r.sha256))
            .collect();

        let native_wasm_metadata = NativeGithubMetadata {
            git: gh.git.clone(),
            tag: gh.tag.clone(),
            bin_hashes,
        };

        if let Err(err) = handle_github_source(
            &native_wasm_metadata,
            shell_config,
            shell_manager_opts,
            *fetcher,
            active_shells,
            logger,
        )
        .await
        {
            logger.warn(err)
        }
    }

    Ok(())
}

async fn handle_github_source(
    service: &NativeGithubMetadata,
    shell_config: &ShellTomlConfig,
    shell_manager_opts: &ShellManagerOpts,
    github: &GithubFetcher,
    active_shells: &mut ActiveShells,
    logger: &DebugLogger,
) -> color_eyre::Result<()> {
    let service_str = utils::get_service_str(service);
    if !active_shells.contains_key(&service_str) {
        // Add in the protocol
        let owner = bytes_to_utf8_string(github.owner.0 .0.clone())?;
        let repo = bytes_to_utf8_string(github.owner.0 .0.clone())?;
        let tag = bytes_to_utf8_string(github.tag.0 .0.clone())?;
        let git = format!("https://github.com/{owner}/{repo}");

        let relevant_binary =
            get_gadget_binary(&github.binaries.0).ok_or_eyre("Unable to find matching binary")?;
        let expected_hash = slice_32_to_sha_hex_string(relevant_binary.sha256);

        let current_dir = std::env::current_dir()?;
        let mut binary_download_path = format!("{}/protocol-{tag}", current_dir.display());

        if utils::is_windows() {
            binary_download_path += ".exe"
        }

        logger.info(format!("Downloading to {binary_download_path}"));

        // Check if the binary exists, if not download it
        let retrieved_hash =
            if !utils::valid_file_exists(&binary_download_path, &expected_hash).await {
                let url = utils::get_download_url(git, &tag);

                let download = reqwest::get(&url)
                    .await
                    .map_err(|err| utils::msg_to_error(err.to_string()))?
                    .bytes()
                    .await
                    .map_err(|err| utils::msg_to_error(err.to_string()))?;
                let retrieved_hash = utils::hash_bytes_to_hex(&download);

                // Write the binary to disk
                let mut file = gadget_io::tokio::fs::File::create(&binary_download_path).await?;
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
                return Ok(());
            }
        }

        if !utils::is_windows() {
            if let Err(err) = utils::chmod_x_file(&binary_download_path).await {
                logger.warn(format!("Failed to chmod +x the binary: {err}"));
            }
        }

        let arguments = utils::generate_process_arguments(shell_config, shell_manager_opts)?;

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

        let (status_handle, abort) =
            utils::generate_running_process_status_handle(process_handle, logger, &service_str);

        active_shells.insert(service_str.clone(), (status_handle, Some(abort)));
    }

    Ok(())
}

fn slice_32_to_sha_hex_string(hash: [u8; 32]) -> String {
    hash.iter().map(|byte| format!("{:02x}", byte)).collect()
}

fn get_gadget_binary(gadget_binaries: &[GadgetBinary]) -> Option<&GadgetBinary> {
    let os = utils::get_formatted_os_string().to_lowercase();
    let arch = std::env::consts::ARCH.to_lowercase();
    for binary in gadget_binaries {
        let binary_str = format!("{:?}", binary.os).to_lowercase();
        if binary_str.contains(&os) || os.contains(&binary_str) || binary_str == os {
            let mut arch_str = format!("{:?}", binary.arch).to_lowercase();

            if arch_str == "amd" {
                arch_str = "x86".to_string()
            } else if arch_str == "amd64" {
                arch_str = "x86_64".to_string()
            }

            if arch_str == arch {
                return Some(binary);
            }
        }
    }

    None
}
