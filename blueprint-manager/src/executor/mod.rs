use crate::config::BlueprintManagerConfig;
use crate::utils;
use color_eyre::Report;
use gadget_common::config::DebugLogger;
use gadget_common::environments::GadgetEnvironment;
use gadget_common::tangle_runtime::AccountId32;
use gadget_io::GadgetConfig;
use shell_sdk::entry::keystore_from_base_path;
use shell_sdk::keystore::load_keys_from_keystore;
use shell_sdk::Client;
use sp_core::Pair;
use std::collections::HashMap;
use tangle_environment::api::ServicesClient;
use tangle_environment::gadget::SubxtConfig;
use tangle_environment::TangleEnvironment;
pub(crate) mod event_handler;

pub async fn run_blueprint_manager(
    blueprint_manager_config: &BlueprintManagerConfig,
    gadget_config: GadgetConfig,
) -> color_eyre::Result<()> {
    let logger_id = if let Some(custom_id) = &blueprint_manager_config.instance_id {
        custom_id.as_str()
    } else {
        "local"
    };

    let logger = &DebugLogger {
        id: format!("blueprint-manager-{}", logger_id),
    };

    let keystore = keystore_from_base_path(
        &gadget_config.base_path,
        gadget_config.chain,
        gadget_config.keystore_password.clone(),
    );

    let (_role_key, acco_key) = load_keys_from_keystore(&keystore)?;
    let sub_account_id = AccountId32(acco_key.public().0);
    let subxt_config = SubxtConfig {
        endpoint: gadget_config.url.clone(),
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

    let (blueprints, init_event) = if let Some(event) = tangle_runtime.next_event().await {
        (
            utils::get_blueprints(&runtime, event.hash, sub_account_id.clone()).await?,
            event,
        )
    } else {
        return Err(Report::msg("Failed to get initial block hash"));
    };

    logger.info(format!("Received {} blueprints", blueprints.len()));
    event_handler::handle_tangle_event(
        &init_event,
        &blueprints,
        logger,
        &gadget_config,
        blueprint_manager_config,
        &mut active_shells,
    )
    .await?;

    let manager_task = async move {
        // Step 2: Listen to FinalityNotifications and poll for new/deleted services that correspond to the blueprints above
        while let Some(event) = tangle_runtime.next_event().await {
            event_handler::handle_tangle_event(
                &event,
                &blueprints,
                logger,
                &gadget_config,
                blueprint_manager_config,
                &mut active_shells,
            )
            .await?;

            event_handler::handle_blueprint_events(
                event,
                logger,
                &mut active_shells,
                &sub_account_id,
            )
            .await;
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
