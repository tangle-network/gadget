use crate::gadget::ActiveShells;
use crate::utils::github_fetcher_to_native_github_metadata;
use color_eyre::Report;
use config::ShellManagerOpts;
use gadget_common::gadget_io;
use gadget_common::prelude::GadgetEnvironment;
use gadget_common::tangle_runtime::api::services::events::PreRegistration;
use gadget_io::ShellTomlConfig;
use shell_sdk::entry::keystore_from_base_path;
use shell_sdk::keystore::load_keys_from_keystore;
use shell_sdk::{entry, Client, DebugLogger};
use sp_core::crypto::Pair;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use structopt::StructOpt;
use tangle_environment::api::{RpcServicesWithBlueprint, ServicesClient};
use tangle_environment::gadget::{SubxtConfig, TangleEvent};
use tangle_environment::TangleEnvironment;
use tangle_subxt::subxt::utils::AccountId32;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::GadgetSourceFetcher;
use tangle_subxt::tangle_testnet_runtime::api::services::events::{
    JobCalled, JobResultSubmitted, Registered, ServiceInitiated, Unregistered,
};

pub mod config;
pub mod error;
pub mod gadget;
pub mod protocols;
pub mod utils;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    //color_eyre::install()?;
    let opt = &ShellManagerOpts::from_args();
    entry::setup_shell_logger(opt.verbose, opt.pretty, "gadget")?;
    let shell_config_contents = std::fs::read_to_string(opt.shell_config.clone())?;
    let shell_config: ShellTomlConfig = toml::from_str(&shell_config_contents)
        .map_err(|err| utils::msg_to_error(err.to_string()))?;
    let logger = &DebugLogger {
        id: "Gadget Shell Manager".into(),
    };

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
    let mut active_shells = HashMap::new();

    // With the basics setup, we must now implement the main logic
    /*
       * Query to get Vec<RpcServicesWithBlueprint>
       * For each RpcServicesWithBlueprint, fetch the associated gadget binary (fetch/download)
        -> If the services field is empty, just emit and log inside the executed binary "that states a new service instance got created by one of these blueprints"
        -> If the services field is not empty, for each service in RpcServicesWithBlueprint.services, spawn the gadget binary, using params to set the job type to listen to (in terms of our old language, each spawned service represents a single "RoleType")
    */

    let (mut blueprints, init_event) = if let Some(event) = tangle_runtime.next_event().await {
        (
            utils::get_blueprints(&runtime, event.hash, sub_account_id.clone()).await?,
            event,
        )
    } else {
        return Err(Report::msg("Failed to get initial block hash"));
    };

    logger.info(format!("Received {} blueprints", blueprints.len()));
    handle_tangle_event(
        &init_event,
        &blueprints,
        logger,
        &shell_config,
        opt,
        &mut active_shells,
    )
    .await?;

    let manager_task = async move {
        // Step 2: Listen to FinalityNotifications and poll for new/deleted services that correspond to the blueprints above
        while let Some(event) = tangle_runtime.next_event().await {
            let result = handle_tangle_block(&event, logger, &mut active_shells, &sub_account_id).await;

            if result.needs_update {
                blueprints = utils::get_blueprints(&runtime, event.hash, sub_account_id.clone()).await?;
            }

            handle_tangle_event(
                &event,
                &blueprints,
                logger,
                &shell_config,
                opt,
                &mut active_shells,
                result,
            )
            .await?;
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

#[derive(Default, Debug)]
struct EventPollResult {
    needs_update: bool,
    // A vec of blueprints we have not yet become registered to
    registration_mode: Vec<u64>,
}

async fn handle_tangle_block(
    event: &TangleEvent,
    logger: &DebugLogger,
    active_shells: &mut ActiveShells,
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
                    result.registration_mode.push(evt.blueprint_id);
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
                if &evt.operator == account_id && active_shells.remove(&evt.blueprint_id).is_some()
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

async fn handle_tangle_event(
    event: &TangleEvent,
    blueprints: &Vec<RpcServicesWithBlueprint>,
    logger: &DebugLogger,
    shell_config: &ShellTomlConfig,
    shell_manager_opts: &ShellManagerOpts,
    active_shells: &mut ActiveShells,
    poll_result: EventPollResult,
) -> color_eyre::Result<()> {
    logger.info(format!("Received notification {}", event.number));
    // TODO: Refactor into Vec<SourceMetadata<T>> where T: NativeGithubMetadata + [...]
    let mut onchain_services = vec![];
    let mut fetchers = vec![];
    let mut service_ids = vec![];

    let mut valid_blueprint_ids = vec![];

    for blueprint in blueprints {
        let mut services_for_this_blueprint = vec![];
        if let runtime_types::tangle_primitives::services::Gadget::Native(gadget) =
            &blueprint.blueprint.gadget
        {
            let gadget_source = &gadget.soruces.0[0];
            if let GadgetSourceFetcher::Github(gh) = &gadget_source.fetcher {
                let metadata = github_fetcher_to_native_github_metadata(gh, blueprint.blueprint_id);
                onchain_services.push(metadata);
                fetchers.push(gh);
                valid_blueprint_ids.push(blueprint.blueprint_id);

                for service in &blueprint.services {
                    services_for_this_blueprint.push(service.id);
                }
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
    gadget::native::maybe_handle(
        blueprints,
        &onchain_services,
        &fetchers,
        shell_config,
        shell_manager_opts,
        active_shells,
        logger,
        poll_result,
    )
    .await?;

    // Check to see if local is running services that are not on-chain
    let mut to_remove: Vec<(u64, u64)> = vec![];

    // Loop through every (blueprint_id, service_id) running. See if the service is still on-chain. If not, kill it and add it to to_remove
    for (blueprint_id, process_handles) in &mut *active_shells {
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
    for (blueprint_id, process_handles) in &mut *active_shells {
        for (service_id, process_handle) in process_handles {
            if !process_handle.0.load(Ordering::Relaxed) {
                // By removing any killed processes, we will auto-restart them on the next finality notification if required
                to_remove.push((*blueprint_id, *service_id));
            }
        }
    }

    for (blueprint_id, service_id) in to_remove {
        let mut should_delete_blueprint = false;
        if let Some(shells) = active_shells.get_mut(&blueprint_id) {
            shells.remove(&service_id);
            if shells.is_empty() {
                should_delete_blueprint = true;
            }
        }

        if should_delete_blueprint {
            active_shells.remove(&blueprint_id);
        }
    }

    Ok(())
}
