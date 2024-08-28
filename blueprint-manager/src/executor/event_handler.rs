use crate::config::BlueprintManagerConfig;
use crate::gadget::native::{get_gadget_binary, FilteredBlueprint};
use crate::gadget::ActiveGadgets;
use crate::protocols::resolver::NativeGithubMetadata;
use crate::sdk::utils::{
    chmod_x_file, generate_process_arguments, generate_running_process_status_handle,
    get_download_url, get_service_str, github_fetcher_to_native_github_metadata, hash_bytes_to_hex,
    is_windows, msg_to_error, valid_file_exists,
};
use color_eyre::eyre::OptionExt;
use gadget_common::prelude::DebugLogger;
use gadget_common::tangle_runtime::AccountId32;
use gadget_io::GadgetConfig;
use std::fmt::Write;
use std::sync::atomic::Ordering;
use tangle_environment::api::{RpcServicesWithBlueprint, ServicesClient};
use tangle_environment::gadget::TangleEvent;
use tangle_subxt::subxt::SubstrateConfig;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::{
    Gadget, GithubFetcher,
};
use tangle_subxt::tangle_testnet_runtime::api::services::events::{
    JobCalled, JobResultSubmitted, PreRegistration, Registered, ServiceInitiated, Unregistered,
};
use tokio::io::AsyncWriteExt;

#[allow(clippy::too_many_arguments)]
pub async fn maybe_handle(
    blueprints: &[FilteredBlueprint],
    onchain_services: &[NativeGithubMetadata],
    onchain_gh_fetchers: &[GithubFetcher],
    gadget_config: &GadgetConfig,
    blueprint_manager_opts: &BlueprintManagerConfig,
    active_gadgets: &mut ActiveGadgets,
    logger: &DebugLogger,
) -> color_eyre::Result<()> {
    for (gh, fetcher) in onchain_services.iter().zip(onchain_gh_fetchers) {
        let native_github_metadata = NativeGithubMetadata {
            git: gh.git.clone(),
            tag: gh.tag.clone(),
            owner: gh.owner.clone(),
            repo: gh.repo.clone(),
            gadget_binaries: fetcher.binaries.0.clone(),
            blueprint_id: gh.blueprint_id,
        };

        if let Err(err) = handle_github_source(
            blueprints,
            &native_github_metadata,
            gadget_config,
            blueprint_manager_opts,
            fetcher,
            active_gadgets,
            logger,
        )
        .await
        {
            logger.warn(err)
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn handle_github_source(
    blueprints: &[FilteredBlueprint],
    service: &NativeGithubMetadata,
    gadget_config: &GadgetConfig,
    blueprint_manager_opts: &BlueprintManagerConfig,
    github: &GithubFetcher,
    active_gadgets: &mut ActiveGadgets,
    logger: &DebugLogger,
) -> color_eyre::Result<()> {
    let blueprint_id = service.blueprint_id;
    let service_str = get_service_str(service);
    if !active_gadgets.contains_key(&blueprint_id) {
        // Maybe add in the protocol to the active shells
        let relevant_binary =
            get_gadget_binary(&github.binaries.0).ok_or_eyre("Unable to find matching binary")?;
        let expected_hash = slice_32_to_sha_hex_string(relevant_binary.sha256);

        let current_dir = std::env::current_dir()?;
        let mut binary_download_path =
            format!("{}/protocol-{:?}", current_dir.display(), github.tag);

        if is_windows() {
            binary_download_path += ".exe"
        }

        logger.info(format!("Downloading to {binary_download_path}"));

        // Check if the binary exists, if not download it
        let retrieved_hash = if !valid_file_exists(&binary_download_path, &expected_hash).await {
            let url = get_download_url(&service.git, &service.tag);

            let download = reqwest::get(&url)
                .await
                .map_err(|err| msg_to_error(err.to_string()))?
                .bytes()
                .await
                .map_err(|err| msg_to_error(err.to_string()))?;
            let retrieved_hash = hash_bytes_to_hex(&download);

            // Write the binary to disk
            let mut file = tokio::fs::File::create(&binary_download_path).await?;
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

        if !is_windows() {
            if let Err(err) = chmod_x_file(&binary_download_path).await {
                logger.warn(format!("Failed to chmod +x the binary: {err}"));
            }
        }

        for blueprint in blueprints {
            if blueprint.blueprint_id == blueprint_id {
                for service_id in &blueprint.services {
                    let sub_service_str = format!("{service_str}-{service_id}");
                    // Each spawned binary will effectively run a single "RoleType" in old parlance
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

fn slice_32_to_sha_hex_string(hash: [u8; 32]) -> String {
    hash.iter().fold(String::new(), |mut acc, byte| {
        write!(&mut acc, "{:02x}", byte).expect("Should be able to write");
        acc
    })
}

#[derive(Default, Debug)]
pub struct EventPollResult {
    pub needs_update: bool,
    // A vec of blueprints we have not yet become registered to
    pub blueprint_registrations: Vec<u64>,
}

pub(crate) async fn check_blueprint_events(
    event: &TangleEvent,
    logger: &DebugLogger,
    active_gadgets: &mut ActiveGadgets,
    account_id: &AccountId32,
) -> EventPollResult {
    let pre_registation_events = event.events.find::<PreRegistration>();
    let registered_events = event.events.find::<Registered>();
    let unregistered_events = event.events.find::<Unregistered>();
    let service_initiated_events = event.events.find::<ServiceInitiated>();
    let job_called_events = event.events.find::<JobCalled>();
    let job_result_submitted_events = event.events.find::<JobResultSubmitted>();

    let mut result = EventPollResult::default();

    for evt in pre_registation_events {
        match evt {
            Ok(evt) => {
                if &evt.operator == account_id {
                    result.blueprint_registrations.push(evt.blueprint_id);
                    logger.info(format!("Pre-registered event: {evt:?}"));
                }
            }
            Err(err) => {
                logger.warn(format!("Error handling pre-registered event: {err:?}"));
            }
        }
    }

    // Handle registered events
    for evt in registered_events {
        match evt {
            Ok(evt) => {
                logger.info(format!("Registered event: {evt:?}"));
                result.needs_update = true;
            }
            Err(err) => {
                logger.warn(format!("Error handling registered event: {err:?}"));
            }
        }
    }

    // Handle unregistered events
    for evt in unregistered_events {
        match evt {
            Ok(evt) => {
                logger.info(format!("Unregistered event: {evt:?}"));
                if &evt.operator == account_id && active_gadgets.remove(&evt.blueprint_id).is_some()
                {
                    logger.info(format!(
                        "Removed services for blueprint_id: {}",
                        evt.blueprint_id,
                    ));

                    result.needs_update = true;
                }
            }
            Err(err) => {
                logger.warn(format!("Error handling unregistered event: {err:?}"));
            }
        }
    }

    // Handle service initiated events
    for evt in service_initiated_events {
        match evt {
            Ok(evt) => {
                logger.info(format!("Service initiated event: {evt:?}"));
            }
            Err(err) => {
                logger.warn(format!("Error handling service initiated event: {err:?}"));
            }
        }
    }

    // Handle job called events
    for evt in job_called_events {
        match evt {
            Ok(evt) => {
                logger.info(format!("Job called event: {evt:?}"));
            }
            Err(err) => {
                logger.warn(format!("Error handling job called event: {err:?}"));
            }
        }
    }

    // Handle job result submitted events
    for evt in job_result_submitted_events {
        match evt {
            Ok(evt) => {
                logger.info(format!("Job result submitted event: {evt:?}"));
            }
            Err(err) => {
                logger.warn(format!(
                    "Error handling job result submitted event: {err:?}"
                ));
            }
        }
    }

    result
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn handle_tangle_event(
    event: &TangleEvent,
    blueprints: &[RpcServicesWithBlueprint],
    logger: &DebugLogger,
    gadget_config: &GadgetConfig,
    gadget_manager_opts: &BlueprintManagerConfig,
    active_gadgets: &mut ActiveGadgets,
    poll_result: EventPollResult,
    client: &ServicesClient<SubstrateConfig>,
) -> color_eyre::Result<()> {
    logger.info(format!("Received notification {}", event.number));

    let mut registration_blueprints = vec![];
    // First, check to see if we need to register any new services invoked by the PreRegistration event
    if !poll_result.blueprint_registrations.is_empty() {
        for blueprint_id in &poll_result.blueprint_registrations {
            let blueprint = client
                .get_blueprint_by_id(event.hash, *blueprint_id)
                .await?
                .ok_or_eyre("Unable to retrieve blueprint for registration mode")?;

            let general_blueprint = FilteredBlueprint {
                blueprint_id: *blueprint_id,
                services: vec![0], // Add a dummy service id for now, since it does not matter for registration mode
                gadget: blueprint.gadget,
                registration_mode: true,
            };

            registration_blueprints.push(general_blueprint);
        }
    }

    // TODO: Refactor into Vec<SourceMetadata<T>> where T: NativeGithubMetadata + [...]
    let mut onchain_services = vec![];
    let mut fetchers = vec![];
    let mut service_ids = vec![];
    let mut valid_blueprint_ids = vec![];

    let mut blueprints_filtered = vec![];

    for blueprint in blueprints
        .iter()
        .map(|r| FilteredBlueprint {
            blueprint_id: r.blueprint_id,
            services: r.services.iter().map(|r| r.id).collect(),
            gadget: r.blueprint.gadget.clone(),
            registration_mode: false,
        })
        .chain(registration_blueprints)
    {
        let mut services_for_this_blueprint = vec![];
        if let Gadget::Native(gadget) = &blueprint.gadget {
            // TODO: fix typo in soruces -> sources
            // needs to update the tangle-subxt to fix the typo
            let gadget_source = &gadget.sources.0[0];
            if let gadget_common::tangle_runtime::api::runtime_types::tangle_primitives::services::GadgetSourceFetcher::Github(gh) = &gadget_source.fetcher {
                let metadata = github_fetcher_to_native_github_metadata(gh, blueprint.blueprint_id);
                onchain_services.push(metadata);
                fetchers.push(gh.clone());
                valid_blueprint_ids.push(blueprint.blueprint_id);

                for service in &blueprint.services {
                    services_for_this_blueprint.push(*service);
                }

                blueprints_filtered.push(blueprint);
            } else {
                logger.warn(
                    "Blueprint does not contain a Github fetcher and thus currently unsupported",
                );
            }

            service_ids.push(services_for_this_blueprint);
        } else {
            logger
                .warn("Blueprint does not contain a native gadget and thus currently unsupported");
        }
    }

    logger.trace(format!(
        "OnChain services: {:?}",
        onchain_services
            .iter()
            .map(|r| r.git.clone())
            .collect::<Vec<_>>()
    ));

    // Step 3: Check to see if we need to start any new services
    maybe_handle(
        &blueprints_filtered,
        &onchain_services,
        &fetchers,
        gadget_config,
        gadget_manager_opts,
        active_gadgets,
        logger,
    )
    .await?;

    // Check to see if local is running services that are not on-chain
    let mut to_remove: Vec<(u64, u64)> = vec![];

    // Loop through every (blueprint_id, service_id) running. See if the service is still on-chain. If not, kill it and add it to to_remove
    for (blueprint_id, process_handles) in &mut *active_gadgets {
        for (service_id, process_handle) in process_handles {
            if !onchain_services
                .iter()
                .zip(&service_ids)
                .all(|(r, s)| r.blueprint_id != *blueprint_id && !s.contains(service_id))
            {
                logger.warn(format!(
                    "Killing service that is no longer on-chain: bid={blueprint_id}//sid={service_id}",
                ));
                if let Some(abort_handle) = process_handle.1.take() {
                    if abort_handle.send(()).is_err() {
                        logger.error(format!(
                            "Failed to send abort signal to service: bid={blueprint_id}//sid={service_id}",
                        ));
                    }
                }

                to_remove.push((*blueprint_id, *service_id));
            }
        }
    }

    // Check to see if any process handles have died
    for (blueprint_id, process_handles) in &mut *active_gadgets {
        for (service_id, process_handle) in process_handles {
            if !process_handle.0.load(Ordering::Relaxed) {
                // By removing any killed processes, we will auto-restart them on the next finality notification if required
                to_remove.push((*blueprint_id, *service_id));
            }
        }
    }

    for (blueprint_id, service_id) in to_remove {
        let mut should_delete_blueprint = false;
        if let Some(gadgets) = active_gadgets.get_mut(&blueprint_id) {
            gadgets.remove(&service_id);
            if gadgets.is_empty() {
                should_delete_blueprint = true;
            }
        }

        if should_delete_blueprint {
            active_gadgets.remove(&blueprint_id);
        }
    }

    Ok(())
}
