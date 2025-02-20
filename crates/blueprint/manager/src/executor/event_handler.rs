use crate::config::BlueprintManagerConfig;
use crate::error::{Error, Result};
use crate::gadget::native::FilteredBlueprint;
use crate::gadget::ActiveGadgets;
use crate::sdk::utils::{
    bounded_string_to_string, generate_running_process_status_handle, make_executable,
};
use crate::sources::github::GithubBinaryFetcher;
use crate::sources::{process_arguments_and_env, BinarySourceFetcher};
use gadget_clients::tangle::client::{TangleConfig, TangleEvent};
use gadget_clients::tangle::services::{RpcServicesWithBlueprint, TangleServicesClient};
use gadget_config::{GadgetConfiguration, Protocol};
use gadget_logging::{error, info, trace, warn};
use std::fmt::Debug;
use std::sync::atomic::Ordering;
use tangle_subxt::subxt::utils::AccountId32;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::gadget::{
    Gadget, GadgetSourceFetcher,
};
use tangle_subxt::tangle_testnet_runtime::api::services::events::{
    JobCalled, JobResultSubmitted, PreRegistration, Registered, ServiceInitiated, Unregistered,
};

const DEFAULT_PROTOCOL: Protocol = Protocol::Tangle;

pub struct VerifiedBlueprint<'a> {
    pub(crate) fetcher: Box<dyn BinarySourceFetcher + 'a>,
    pub(crate) blueprint: FilteredBlueprint,
}

impl VerifiedBlueprint<'_> {
    pub async fn start_services_if_needed(
        &self,
        gadget_config: &GadgetConfiguration,
        blueprint_manager_opts: &BlueprintManagerConfig,
        active_gadgets: &mut ActiveGadgets,
    ) -> Result<()> {
        let blueprint_source = &self.fetcher;
        let blueprint = &self.blueprint;

        let blueprint_id = blueprint_source.blueprint_id();
        if active_gadgets.contains_key(&blueprint_id) {
            return Ok(());
        }

        let mut binary_download_path = blueprint_source.get_binary().await?;

        // Ensure the binary is executable
        if cfg!(target_family = "windows") {
            if binary_download_path.extension().is_none() {
                binary_download_path.set_extension("exe");
            }
        } else if let Err(err) = make_executable(&binary_download_path) {
            let msg = format!("Failed to make the binary executable: {err}");
            warn!("{}", msg);
            return Err(Error::Other(msg));
        }

        let service_str = blueprint_source.name();
        for service_id in &blueprint.services {
            let sub_service_str = format!("{service_str}-{service_id}");
            let (arguments, env_vars) = process_arguments_and_env(
                gadget_config,
                blueprint_manager_opts,
                blueprint_id,
                *service_id,
                blueprint,
                &sub_service_str,
            );

            info!("Starting protocol: {sub_service_str} with args: {arguments:?}");

            // Now that the file is loaded, spawn the process
            let process_handle = tokio::process::Command::new(&binary_download_path)
                .kill_on_drop(true)
                .stdin(std::process::Stdio::null())
                .current_dir(&std::env::current_dir()?)
                .envs(env_vars)
                .args(arguments)
                .spawn()?;

            if blueprint.registration_mode {
                // We must wait for the process to exit successfully
                let status = process_handle.wait_with_output().await?;
                if status.status.success() {
                    info!("***Protocol (registration mode) {sub_service_str} executed successfully***");
                } else {
                    error!(
                        "Protocol (registration mode) {sub_service_str} failed to execute: {status:?}"
                    );
                }
                continue;
            }

            // A normal running gadget binary. Store the process handle and let the event loop handle the rest

            let (status_handle, abort) =
                generate_running_process_status_handle(process_handle, &sub_service_str);

            active_gadgets
                .entry(blueprint_id)
                .or_default()
                .insert(*service_id, (status_handle, Some(abort)));
        }

        Ok(())
    }
}

impl Debug for VerifiedBlueprint<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        format!(
            "{}/bid={}/sid(s)={:?}",
            self.blueprint.name, self.blueprint.blueprint_id, self.blueprint.services
        )
        .fmt(f)
    }
}

#[derive(Default, Debug)]
pub struct EventPollResult {
    pub needs_update: bool,
    // A vec of blueprints we have not yet become registered to
    pub blueprint_registrations: Vec<u64>,
}

pub(crate) fn check_blueprint_events(
    event: &TangleEvent,
    active_gadgets: &mut ActiveGadgets,
    account_id: &AccountId32,
) -> EventPollResult {
    let pre_registration_events = event.events.find::<PreRegistration>();
    let registered_events = event.events.find::<Registered>();
    let unregistered_events = event.events.find::<Unregistered>();
    let service_initiated_events = event.events.find::<ServiceInitiated>();
    let job_called_events = event.events.find::<JobCalled>();
    let job_result_submitted_events = event.events.find::<JobResultSubmitted>();

    let mut result = EventPollResult::default();

    for evt in pre_registration_events {
        match evt {
            Ok(evt) => {
                if &evt.operator == account_id {
                    result.blueprint_registrations.push(evt.blueprint_id);
                    info!("Pre-registered event: {evt:?}");
                }
            }
            Err(err) => {
                warn!("Error handling pre-registered event: {err:?}");
            }
        }
    }

    // Handle registered events
    for evt in registered_events {
        match evt {
            Ok(evt) => {
                info!("Registered event: {evt:?}");
                result.needs_update = true;
            }
            Err(err) => {
                warn!("Error handling registered event: {err:?}");
            }
        }
    }

    // Handle unregistered events
    for evt in unregistered_events {
        match evt {
            Ok(evt) => {
                info!("Unregistered event: {evt:?}");
                if &evt.operator == account_id && active_gadgets.remove(&evt.blueprint_id).is_some()
                {
                    info!("Removed services for blueprint_id: {}", evt.blueprint_id,);

                    result.needs_update = true;
                }
            }
            Err(err) => {
                warn!("Error handling unregistered event: {err:?}");
            }
        }
    }

    // Handle service initiated events
    for evt in service_initiated_events {
        match evt {
            Ok(evt) => {
                info!("Service initiated event: {evt:?}");
            }
            Err(err) => {
                warn!("Error handling service initiated event: {err:?}");
            }
        }
    }

    // Handle job called events
    for evt in job_called_events {
        match evt {
            Ok(evt) => {
                info!("Job called event: {evt:?}");
            }
            Err(err) => {
                warn!("Error handling job called event: {err:?}");
            }
        }
    }

    // Handle job result submitted events
    for evt in job_result_submitted_events {
        match evt {
            Ok(evt) => {
                info!("Job result submitted event: {evt:?}");
            }
            Err(err) => {
                warn!("Error handling job result submitted event: {err:?}");
            }
        }
    }

    result
}

#[allow(clippy::too_many_lines)]
pub(crate) async fn handle_tangle_event(
    event: &TangleEvent,
    blueprints: &[RpcServicesWithBlueprint],
    gadget_config: &GadgetConfiguration,
    manager_opts: &BlueprintManagerConfig,
    active_gadgets: &mut ActiveGadgets,
    poll_result: EventPollResult,
    client: &TangleServicesClient<TangleConfig>,
) -> Result<()> {
    info!("Received notification {}", event.number);

    let mut registration_blueprints = vec![];
    // First, check to see if we need to register any new services invoked by the PreRegistration event
    if !poll_result.blueprint_registrations.is_empty() {
        for blueprint_id in &poll_result.blueprint_registrations {
            let blueprint = client
                .get_blueprint_by_id(event.hash, *blueprint_id)
                .await?
                .ok_or_else(|| {
                    Error::Other(String::from(
                        "Unable to retrieve blueprint for registration mode",
                    ))
                })?;

            let general_blueprint = FilteredBlueprint {
                blueprint_id: *blueprint_id,
                services: vec![0], // Add a dummy service id for now, since it does not matter for registration mode
                gadget: blueprint.gadget,
                name: bounded_string_to_string(&blueprint.metadata.name)?,
                registration_mode: true,
                protocol: DEFAULT_PROTOCOL,
            };

            registration_blueprints.push(general_blueprint);
        }
    }

    let mut verified_blueprints = vec![];

    for blueprint in blueprints
        .iter()
        .map(|r| FilteredBlueprint {
            blueprint_id: r.blueprint_id,
            services: r.services.iter().map(|r| r.id).collect(),
            gadget: r.blueprint.gadget.clone(),
            name: bounded_string_to_string(&r.blueprint.metadata.name)
                .unwrap_or("unknown_blueprint_name".to_string()),
            registration_mode: false,
            protocol: DEFAULT_PROTOCOL,
        })
        .chain(registration_blueprints)
    {
        let mut fetcher_candidates = get_fetcher_candidates(&blueprint, manager_opts)?;

        let verified_blueprint = VerifiedBlueprint {
            fetcher: fetcher_candidates.pop().expect("Should exist"),
            blueprint,
        };

        verified_blueprints.push(verified_blueprint);
    }

    trace!(
        "OnChain Verified Blueprints: {:?}",
        verified_blueprints
            .iter()
            .map(|r| format!("{r:?}"))
            .collect::<Vec<_>>()
    );

    // Step 3: Check to see if we need to start any new services
    for blueprint in &verified_blueprints {
        blueprint
            .start_services_if_needed(gadget_config, manager_opts, active_gadgets)
            .await?;
    }

    // Check to see if local is running services that are not on-chain
    let mut to_remove: Vec<(u64, u64)> = vec![];

    // Loop through every (blueprint_id, service_id) running. See if the service is still on-chain. If not, kill it and add it to to_remove
    for (blueprint_id, process_handles) in &mut *active_gadgets {
        for (service_id, process_handle) in process_handles {
            info!(
                "Checking service for on-chain termination: bid={blueprint_id}//sid={service_id}"
            );

            // Since the below "verified blueprints" were freshly obtained from an on-chain source,
            // we compare all these fresh values to see if we're running a service locally that is no longer on-chain
            for verified_blueprints in &verified_blueprints {
                let services = &verified_blueprints.blueprint.services;
                // Safe assertion since we know there is at least one fetcher. All fetchers should have the same blueprint id
                let fetcher = &verified_blueprints.fetcher;
                if fetcher.blueprint_id() == *blueprint_id && !services.contains(service_id) {
                    warn!("Killing service that is no longer on-chain: bid={blueprint_id}//sid={service_id}");
                    to_remove.push((*blueprint_id, *service_id));
                }
            }

            // Check to see if any process handles have died
            if !to_remove.contains(&(*blueprint_id, *service_id))
                && !process_handle.0.load(Ordering::Relaxed)
            {
                // By removing any killed processes, we will auto-restart them on the next finality notification if required
                warn!("Killing service that has died to allow for auto-restart");
                to_remove.push((*blueprint_id, *service_id));
            }
        }
    }

    for (blueprint_id, service_id) in to_remove {
        warn!("Removing service that is no longer active on-chain or killed: bid={blueprint_id}//sid={service_id}");
        let mut should_delete_blueprint = false;
        if let Some(gadgets) = active_gadgets.get_mut(&blueprint_id) {
            if let Some((_, mut process_handle)) = gadgets.remove(&service_id) {
                if let Some(abort_handle) = process_handle.take() {
                    if abort_handle.send(()).is_err() {
                        error!("Failed to send abort signal to service: bid={blueprint_id}//sid={service_id}");
                    } else {
                        warn!("Sent abort signal to service: bid={blueprint_id}//sid={service_id}");
                    }
                }
            }

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

fn get_fetcher_candidates(
    blueprint: &FilteredBlueprint,
    manager_opts: &BlueprintManagerConfig,
) -> Result<Vec<Box<dyn BinarySourceFetcher>>> {
    let mut test_fetcher_idx = None;
    let mut fetcher_candidates: Vec<Box<dyn BinarySourceFetcher>> = vec![];

    let sources;
    match &blueprint.gadget {
        Gadget::Native(gadget) => {
            sources = &gadget.sources.0;
        }
        Gadget::Wasm(_) => {
            warn!("WASM gadgets are not supported yet");
            return Err(Error::UnsupportedGadget);
        }
        Gadget::Container(_) => {
            warn!("Container gadgets are not supported yet");
            return Err(Error::UnsupportedGadget);
        }
    }

    for (source_idx, gadget_source) in sources.iter().enumerate() {
        match &gadget_source.fetcher {
            GadgetSourceFetcher::Github(gh) => {
                let fetcher = GithubBinaryFetcher {
                    fetcher: gh.clone(),
                    blueprint_id: blueprint.blueprint_id,
                    gadget_name: blueprint.name.clone(),
                };

                fetcher_candidates.push(Box::new(fetcher));
            }

            GadgetSourceFetcher::Testing(test) => {
                // TODO: demote to TRACE once proven to work
                if !manager_opts.test_mode {
                    warn!("Ignoring testing fetcher as we are not in test mode");
                    continue;
                }

                let fetcher = crate::sources::testing::TestSourceFetcher {
                    fetcher: test.clone(),
                    blueprint_id: blueprint.blueprint_id,
                    gadget_name: blueprint.name.clone(),
                };

                test_fetcher_idx = Some(source_idx);
                fetcher_candidates.push(Box::new(fetcher));
            }

            _ => {
                warn!("Blueprint does not contain a supported fetcher");
            }
        }
    }

    // A bunch of sanity checks to enforce structure

    // Ensure that we have at least one fetcher
    if fetcher_candidates.is_empty() {
        return Err(Error::NoFetchers);
    }

    // Ensure there is only a single candidate fetcher
    if fetcher_candidates.len() != 1 {
        return Err(Error::MultipleFetchers);
    }

    // Ensure that we have a test fetcher if we are in test mode
    if manager_opts.test_mode && test_fetcher_idx.is_none() {
        return Err(Error::NoTestFetcher);
    }

    // Ensure that we have only one fetcher if we are in test mode
    if manager_opts.test_mode {
        fetcher_candidates =
            vec![fetcher_candidates.remove(test_fetcher_idx.expect("Should exist"))];
    }

    Ok(fetcher_candidates)
}
