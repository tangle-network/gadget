use crate::config::BlueprintManagerConfig;
use crate::gadget::native::FilteredBlueprint;
use crate::gadget::ActiveGadgets;
use crate::sdk::utils::bounded_string_to_string;
use crate::sources::github::GithubBinaryFetcher;
use crate::sources::BinarySourceFetcher;
use color_eyre::eyre::OptionExt;
use gadget_io::GadgetConfig;
use gadget_sdk::clients::tangle::runtime::TangleEvent;
use gadget_sdk::clients::tangle::services::{RpcServicesWithBlueprint, ServicesClient};
use gadget_sdk::logger::Logger;
use itertools::Itertools;
use std::sync::atomic::Ordering;
use tangle_subxt::subxt::utils::AccountId32;
use tangle_subxt::subxt::PolkadotConfig;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::{
    Gadget, GadgetSourceFetcher,
};
use tangle_subxt::tangle_testnet_runtime::api::services::events::{
    JobCalled, JobResultSubmitted, PreRegistration, Registered, ServiceInitiated, Unregistered,
};

pub async fn handle_services<'a>(
    blueprints: &[FilteredBlueprint],
    fetchers: &[Box<dyn BinarySourceFetcher + 'a>],
    gadget_config: &GadgetConfig,
    blueprint_manager_opts: &BlueprintManagerConfig,
    active_gadgets: &mut ActiveGadgets,
    logger: &Logger,
) -> color_eyre::Result<()> {
    for fetcher in fetchers {
        if let Err(err) = crate::sources::handle(
            fetcher,
            blueprints,
            gadget_config,
            blueprint_manager_opts,
            active_gadgets,
            logger,
        )
        .await
        {
            logger.error(err)
        }
    }

    Ok(())
}

#[derive(Default, Debug)]
pub struct EventPollResult {
    pub needs_update: bool,
    // A vec of blueprints we have not yet become registered to
    pub blueprint_registrations: Vec<u64>,
}

pub(crate) async fn check_blueprint_events(
    event: &TangleEvent,
    logger: &Logger,
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
    logger: &Logger,
    gadget_config: &GadgetConfig,
    gadget_manager_opts: &BlueprintManagerConfig,
    active_gadgets: &mut ActiveGadgets,
    poll_result: EventPollResult,
    client: &ServicesClient<PolkadotConfig>,
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
                name: bounded_string_to_string(blueprint.metadata.name)?,
                registration_mode: true,
            };

            registration_blueprints.push(general_blueprint);
        }
    }

    let mut fetchers = vec![];
    let mut service_ids = vec![];
    let mut blueprints_filtered = vec![];

    for blueprint in blueprints
        .iter()
        .map(|r| FilteredBlueprint {
            blueprint_id: r.blueprint_id,
            services: r.services.iter().map(|r| r.id).collect(),
            gadget: r.blueprint.gadget.clone(),
            name: bounded_string_to_string(r.clone().blueprint.metadata.name)
                .unwrap_or("unknown_blueprint_name".to_string()),
            registration_mode: false,
        })
        .chain(registration_blueprints)
    {
        if let Gadget::Native(gadget) = &blueprint.gadget {
            let gadget_source = &gadget.sources.0[0];
            match &gadget_source.fetcher {
                GadgetSourceFetcher::Github(gh) => {
                    let fetcher = GithubBinaryFetcher {
                        fetcher: gh.clone(),
                        blueprint_id: blueprint.blueprint_id,
                        logger,
                        gadget_name: blueprint.name.clone(),
                    };

                    fetchers.push(Box::new(fetcher) as Box<dyn BinarySourceFetcher>);
                }

                GadgetSourceFetcher::Testing(test) => {
                    let fetcher = crate::sources::testing::TestSourceFetcher {
                        fetcher: test.clone(),
                        blueprint_id: blueprint.blueprint_id,
                        logger,
                        gadget_name: blueprint.name.clone(),
                    };

                    fetchers.push(Box::new(fetcher));
                }

                _ => {
                    logger.warn("Blueprint does not contain a supported fetcher");
                    continue;
                }
            }

            let mut services_for_this_blueprint = vec![];
            for service in &blueprint.services {
                services_for_this_blueprint.push(*service);
            }

            blueprints_filtered.push(blueprint);
            service_ids.push(services_for_this_blueprint);
        } else {
            logger
                .warn("Blueprint does not contain a native gadget and thus currently unsupported");
        }
    }

    logger.trace(format!(
        "OnChain services: {:?}",
        fetchers
            .iter()
            .map(|r| format!("{}/{}", r.blueprint_id(), r.name()))
            .collect::<Vec<_>>()
    ));

    // Step 3: Check to see if we need to start any new services
    handle_services(
        &blueprints_filtered,
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
        for service_id in process_handles.keys() {
            logger.info(format!(
                "Checking service for on-chain termination: bid={blueprint_id}//sid={service_id}"
            ));
            for (fetcher, services) in fetchers.iter().zip_eq(&service_ids) {
                if fetcher.blueprint_id() == *blueprint_id && !services.contains(service_id) {
                    logger.warn(format!(
                        "Killing service that is no longer on-chain: bid={blueprint_id}//sid={service_id}",
                    ));
                    to_remove.push((*blueprint_id, *service_id));
                }
            }
        }
    }

    // Check to see if any process handles have died
    for (blueprint_id, process_handles) in &mut *active_gadgets {
        for (service_id, process_handle) in process_handles {
            if !to_remove.contains(&(*blueprint_id, *service_id))
                && !process_handle.0.load(Ordering::Relaxed)
            {
                // By removing any killed processes, we will auto-restart them on the next finality notification if required
                logger.warn("Killing service that has died to allow for auto-restart");
                to_remove.push((*blueprint_id, *service_id));
            }
        }
    }

    for (blueprint_id, service_id) in to_remove {
        logger.warn(format!(
            "Removing service that is no longer active on-chain or killed: bid={blueprint_id}//sid={service_id}",
        ));
        let mut should_delete_blueprint = false;
        if let Some(gadgets) = active_gadgets.get_mut(&blueprint_id) {
            if let Some((_, mut process_handle)) = gadgets.remove(&service_id) {
                if let Some(abort_handle) = process_handle.take() {
                    if abort_handle.send(()).is_err() {
                        logger.error(format!(
                            "Failed to send abort signal to service: bid={blueprint_id}//sid={service_id}",
                        ));
                    } else {
                        logger.warn(format!(
                            "Sent abort signal to service: bid={blueprint_id}//sid={service_id}",
                        ));
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
