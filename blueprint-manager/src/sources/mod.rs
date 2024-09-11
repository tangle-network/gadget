use crate::config::BlueprintManagerConfig;
use crate::gadget::native::FilteredBlueprint;
use crate::gadget::ActiveGadgets;
use crate::sdk::utils::{
    chmod_x_file, generate_process_arguments, generate_running_process_status_handle, is_windows,
};
use async_trait::async_trait;
use gadget_io::GadgetConfig;
use gadget_sdk::logger::Logger;
use std::path::PathBuf;

pub mod github;
pub mod testing;

#[async_trait]
#[auto_impl::auto_impl(Box)]
pub trait BinarySourceFetcher: Send + Sync {
    async fn get_binary(&self) -> color_eyre::Result<PathBuf>;
    fn blueprint_id(&self) -> u64;
    fn name(&self) -> String;
}

pub async fn handle(
    service_source: &dyn BinarySourceFetcher,
    blueprints: &[FilteredBlueprint],
    gadget_config: &GadgetConfig,
    blueprint_manager_opts: &BlueprintManagerConfig,
    active_gadgets: &mut ActiveGadgets,
    logger: &Logger,
) -> color_eyre::Result<()> {
    let blueprint_id = service_source.blueprint_id();
    let service_str = service_source.name();

    if !active_gadgets.contains_key(&blueprint_id) {
        let mut binary_download_path = service_source.get_binary().await?;

        // Ensure the binary is executable
        if is_windows() {
            if binary_download_path.extension().is_none() {
                binary_download_path.set_extension("exe");
            }
        } else if let Err(err) = chmod_x_file(&binary_download_path).await {
            logger.warn(format!("Failed to chmod +x the binary: {err}"));
        }

        for blueprint in blueprints {
            if blueprint.blueprint_id == blueprint_id {
                for service_id in &blueprint.services {
                    let sub_service_str = format!("{service_str}-{service_id}");
                    let arguments = generate_process_arguments(
                        gadget_config,
                        blueprint_manager_opts,
                        blueprint_id,
                        *service_id,
                    )?;

                    let env_vars = if blueprint.registration_mode {
                        let mut env_vars = std::env::vars().collect::<Vec<_>>();
                        // RPC_URL: The remote RPC url for tangle
                        // KEYSTORE_URI: the keystore file where the keys are stored and could be retrieved.
                        // DATA_DIR: is an isolated path for where this gadget should store its data (database, secrets, …etc)
                        // BLUEPRINT_ID: the active blueprint ID for this gadget
                        // REGISTRATION_MODE_ON set to any value.
                        env_vars.push(("RPC_URL".to_string(), gadget_config.url.to_string()));
                        env_vars.push((
                            "KEYSTORE_URI".to_string(),
                            blueprint_manager_opts.keystore_uri.clone(),
                        ));
                        env_vars.push((
                            "DATA_DIR".to_string(),
                            format!("{}", gadget_config.base_path.display()),
                        ));
                        env_vars.push(("BLUEPRINT_ID".to_string(), format!("{}", blueprint_id)));
                        env_vars.push(("REGISTRATION_MODE_ON".to_string(), "true".to_string()));
                        env_vars
                    } else {
                        std::env::vars().collect::<Vec<_>>()
                    };

                    logger.info(format!("Starting protocol: {sub_service_str}"));

                    // Now that the file is loaded, spawn the process
                    let process_handle = tokio::process::Command::new(&binary_download_path)
                        .kill_on_drop(true)
                        .stdout(std::process::Stdio::inherit()) // Inherit the stdout of this process
                        .stderr(std::process::Stdio::inherit()) // Inherit the stderr of this process
                        .stdin(std::process::Stdio::null())
                        .current_dir(&std::env::current_dir()?)
                        .envs(env_vars)
                        .args(arguments)
                        .spawn()?;

                    if blueprint.registration_mode {
                        // We must wait for the process to exit successfully
                        let status = process_handle.wait_with_output().await?;
                        if !status.status.success() {
                            logger.error(format!(
                                "Protocol (registration mode) {sub_service_str} failed to execute: {status:?}"
                            ));
                        } else {
                            logger.info(format!(
                                "***Protocol (registration mode) {sub_service_str} executed successfully***"
                            ));
                        }
                    } else {
                        // A normal running gadget binary. Store the process handle and let the event loop handle the rest
                        let (status_handle, abort) = generate_running_process_status_handle(
                            process_handle,
                            logger,
                            &sub_service_str,
                        );

                        active_gadgets
                            .entry(blueprint_id)
                            .or_default()
                            .insert(*service_id, (status_handle, Some(abort)));
                    }
                }
            }
        }
    }

    Ok(())
}
