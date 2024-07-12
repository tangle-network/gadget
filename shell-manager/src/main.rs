use crate::protocols::resolver::{load_global_config_file, NativeGithubMetadata};
use color_eyre::Report;
use config::ShellManagerOpts;
use gadget_common::gadget_io;
use gadget_common::prelude::GadgetEnvironment;
use gadget_common::sp_core::Pair;
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
pub mod gadget;
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

    // Get blueprint on inti, per ordering requirement
    let blueprints = if let Some(event) = tangle_runtime.next_event().await {
        utils::get_blueprints(
            &runtime,
            event.hash,
            sub_account_id.clone(),
            &global_protocols,
            test_mode,
        )
        .await?
    } else {
        return Err(Report::msg("Failed to get initial block hash"));
    };
    
    
    let manager_task = async move {
        // Step 2: Listen to FinalityNotifications and poll for new services that correspond to the blueprints above
        while let Some(event) = tangle_runtime.next_event().await {
            logger.info(format!("Received notification {}", event.number));
            let onchain_services = utils::get_services(
                &runtime,
                event.hash,
                sub_account_id.clone(),
                &global_protocols,
                test_mode,
            )
            .await?;
            let onchain_services = onchain_services
                .iter()
                .map(|(fetcher, metadata)| metadata.clone())
                .collect::<Vec<_>>();

            let fetchers = onchain_services
                .iter()
                .map(|fetcher, _| *fetcher)
                .collect::<Vec<&NativeGithubMetadata>>();

            logger.trace(format!(
                "OnChain services: {:?}",
                onchain_services
                    .iter()
                    .map(|r| r.git.clone())
                    .collect::<Vec<_>>()
            ));

            // Step 3: Check to see if we need to start any new services
            gadget::native::handle(
                &onchain_services,
                fetchers,
                &shell_config,
                opt,
                &mut active_shells,
                &global_protocols,
                logger,
            )
            .await?;

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
