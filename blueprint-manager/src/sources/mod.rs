use crate::config::BlueprintManagerConfig;
use crate::executor::event_handler::VerifiedBlueprint;
use crate::gadget::ActiveGadgets;
use crate::sdk::utils::{
    chmod_x_file, generate_process_arguments, generate_running_process_status_handle, is_windows,
};
use async_trait::async_trait;
use gadget_io::GadgetConfig;
use gadget_sdk::{error, info, warn};
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

pub async fn handle<'a>(
    blueprint: &VerifiedBlueprint<'a>,
    gadget_config: &GadgetConfig,
    blueprint_manager_opts: &BlueprintManagerConfig,
    active_gadgets: &mut ActiveGadgets,
) -> color_eyre::Result<()> {
    let blueprint_source = &blueprint.fetcher;
    let blueprint = &blueprint.blueprint;

    let blueprint_id = blueprint_source.blueprint_id();
    let service_str = blueprint_source.name();

    if !active_gadgets.contains_key(&blueprint_id) {
        let mut binary_download_path = blueprint_source.get_binary().await?;

        // Ensure the binary is executable
        if is_windows() {
            if binary_download_path.extension().is_none() {
                binary_download_path.set_extension("exe");
            }
        } else if let Err(err) = chmod_x_file(&binary_download_path).await {
            warn!("Failed to chmod +x the binary: {err}");
        }

        for service_id in &blueprint.services {
            let sub_service_str = format!("{service_str}-{service_id}");
            let arguments = generate_process_arguments(
                gadget_config,
                blueprint_manager_opts,
                blueprint_id,
                *service_id,
                blueprint.protocol,
            )?;

            // Add required env vars for all child processes/gadgets
            let mut env_vars = vec![
                ("RPC_URL".to_string(), gadget_config.url.to_string()),
                (
                    "KEYSTORE_URI".to_string(),
                    blueprint_manager_opts.keystore_uri.clone(),
                ),
                ("DATA_DIR".to_string(), gadget_config.keystore_uri.clone()),
                ("BLUEPRINT_ID".to_string(), format!("{}", blueprint_id)),
                ("SERVICE_ID".to_string(), format!("{}", service_id)),
            ];

            // Ensure our child process inherits the current processes' environment vars
            env_vars.extend(std::env::vars());

            if blueprint.registration_mode {
                env_vars.push(("REGISTRATION_MODE_ON".to_string(), "true".to_string()));
            }

            info!("Starting protocol: {sub_service_str} with args: {arguments:?}");

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
                    error!(
                        "Protocol (registration mode) {sub_service_str} failed to execute: {status:?}"
                    );
                } else {
                    info!(
                        "***Protocol (registration mode) {sub_service_str} executed successfully***"
                    );
                }
            } else {
                // A normal running gadget binary. Store the process handle and let the event loop handle the rest

                let (status_handle, abort) =
                    generate_running_process_status_handle(process_handle, &sub_service_str);

                active_gadgets
                    .entry(blueprint_id)
                    .or_default()
                    .insert(*service_id, (status_handle, Some(abort)));
            }
        }
    }

    Ok(())
}
