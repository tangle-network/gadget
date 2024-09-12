use crate::config::BlueprintManagerConfig;
use crate::gadget::ActiveGadgets;
use crate::sdk::entry::{keystore_from_base_path, SendFuture};
use crate::sdk::utils;
use crate::sdk::utils::msg_to_error;
use color_eyre::eyre::OptionExt;
use color_eyre::Report;
use gadget_io::GadgetConfig;
use gadget_io::SubstrateKeystore;
use gadget_sdk::clients::tangle::runtime::TangleRuntimeClient;
use gadget_sdk::clients::tangle::services::{RpcServicesWithBlueprint, ServicesClient};
use gadget_sdk::clients::Client;
use gadget_sdk::keystore::backend::fs::FilesystemKeystore;
use gadget_sdk::keystore::backend::GenericKeyStore;
use gadget_sdk::keystore::{BackendExt, TanglePairSigner};
use gadget_sdk::logger::Logger;
use sp_core::H256;
use sp_core::{ecdsa, Pair};
use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};
use tangle_subxt::subxt::blocks::BlockRef;
use tangle_subxt::subxt::utils::AccountId32;
use tangle_subxt::subxt::{Config, SubstrateConfig};
use tangle_subxt::subxt_signer;
use tangle_subxt::subxt_signer::sr25519;
use tokio::task::JoinHandle;

pub(crate) mod event_handler;

pub struct BlueprintManagerHandle {
    shutdown_call: Option<tokio::sync::oneshot::Sender<()>>,
    start_tx: Option<tokio::sync::oneshot::Sender<()>>,
    running_task: JoinHandle<color_eyre::Result<()>>,
    logger: Logger,
    sr25519_id: TanglePairSigner,
    ecdsa_id: subxt_signer::ecdsa::Keypair,
    keystore_path: PathBuf,
}

impl BlueprintManagerHandle {
    /// Send a start signal to the blueprint manager
    pub fn start(&mut self) -> color_eyre::Result<()> {
        match self.start_tx.take() {
            Some(tx) => match tx.send(()) {
                Ok(_) => {
                    self.logger.info("Start signal sent to Blueprint Manager");
                    Ok(())
                }
                Err(_) => Err(Report::msg(
                    "Failed to send start signal to Blueprint Manager",
                )),
            },
            None => Err(Report::msg("Blueprint Manager Already Started")),
        }
    }

    /// Returns the SR25519 keypair for this blueprint manager
    pub fn sr25519_id(&self) -> &TanglePairSigner {
        &self.sr25519_id
    }

    /// Returns the ECDSA keypair for this blueprint manager
    pub fn ecdsa_id(&self) -> &subxt_signer::ecdsa::Keypair {
        &self.ecdsa_id
    }

    /// Shutdown the blueprint manager
    pub async fn shutdown(&mut self) -> color_eyre::Result<()> {
        self.shutdown_call
            .take()
            .map(|tx| tx.send(()))
            .ok_or_eyre("Shutdown already called")?
            .map_err(|_| Report::msg("Failed to send shutdown signal to Blueprint Manager"))
    }

    /// Returns the logger for this blueprint manager
    pub fn logger(&self) -> &Logger {
        &self.logger
    }

    /// Returns the keystore path for this blueprint manager
    pub fn keystore_path(&self) -> &PathBuf {
        &self.keystore_path
    }
}

/// Add default behavior for unintentional dropping of the BlueprintManagerHandle
/// This will ensure that the BlueprintManagerHandle is executed even if the handle
/// is dropped, which is similar behavior to the tokio SpawnHandle
impl Drop for BlueprintManagerHandle {
    fn drop(&mut self) {
        let _ = self.start();
    }
}

/// Implement the Future trait for the BlueprintManagerHandle to allow
/// for the handle to be awaited on
impl Future for BlueprintManagerHandle {
    type Output = color_eyre::Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Start the blueprint manager if it has not been started
        let this = self.get_mut();
        if this.start_tx.is_some() {
            if let Err(err) = this.start() {
                return Poll::Ready(Err(err));
            }
        }

        let result = futures::ready!(Pin::new(&mut this.running_task).poll(cx));

        match result {
            Ok(res) => Poll::Ready(res),
            Err(err) => Poll::Ready(Err(Report::msg(format!(
                "Blueprint Manager Closed Unexpectedly (JoinError): {err:?}"
            )))),
        }
    }
}

pub async fn run_blueprint_manager<F: SendFuture<'static, ()>>(
    blueprint_manager_config: BlueprintManagerConfig,
    gadget_config: GadgetConfig,
    shutdown_cmd: F,
) -> color_eyre::Result<BlueprintManagerHandle> {
    let logger_id = if let Some(custom_id) = &blueprint_manager_config.instance_id {
        custom_id.as_str()
    } else {
        "Local"
    };

    let logger = Logger {
        target: "blueprint-manager".into(),
        id: format!("Blueprint-Manager-{}", logger_id),
    };

    logger.info("Starting blueprint manager ... waiting for start signal ...");

    let logger_clone = logger.clone();

    let (tangle_key, ecdsa_key) = {
        let keystore = GenericKeyStore::<parking_lot::RawRwLock>::Fs(FilesystemKeystore::open(
            &gadget_config.base_path,
        )?);
        logger.info("Keystore opened successfully");
        let sr_key = keystore.sr25519_key()?;
        logger.info("SR25519 key found");
        let ecdsa_key = keystore.ecdsa_key()?;
        logger.info("ECDSA key found");
        (sr_key, ecdsa_key)
    };

    let sub_account_id = tangle_key.account_id().clone();

    let tangle_client =
        TangleRuntimeClient::from_url(gadget_config.url.as_str(), sub_account_id.clone()).await?;
    let services_client = ServicesClient::new(logger.clone(), tangle_client.client());
    let mut active_gadgets = HashMap::new();

    let base_path = gadget_config.base_path.clone();

    let logger_manager = logger.clone();
    let manager_task = async move {
        // With the basics setup, we must now implement the main logic of the Blueprint Manager
        // Handle initialization logic
        // NOTE: The node running this code should be registered as an operator for the blueprints, otherwise, this
        // code will fail
        let mut operator_subscribed_blueprints = handle_init(
            &tangle_client,
            &services_client,
            &logger_manager,
            &sub_account_id,
            &mut active_gadgets,
            &gadget_config,
            &blueprint_manager_config,
        )
        .await?;

        // Now, run the main event loop
        // Listen to FinalityNotifications and poll for new/deleted services that correspond to the blueprints above
        while let Some(event) = tangle_client.next_event().await {
            let result = event_handler::check_blueprint_events(
                &event,
                &logger_manager,
                &mut active_gadgets,
                &sub_account_id.clone(),
            )
            .await;

            if result.needs_update {
                operator_subscribed_blueprints = services_client
                    .query_operator_blueprints(event.hash, sub_account_id.clone())
                    .await
                    .map_err(|err| msg_to_error(err.to_string()))?;
            }

            event_handler::handle_tangle_event(
                &event,
                &operator_subscribed_blueprints,
                &logger_manager,
                &gadget_config,
                &blueprint_manager_config,
                &mut active_gadgets,
                result,
                &services_client,
            )
            .await?;
        }

        Err::<(), _>(utils::msg_to_error("Finality Notification stream died"))
    };

    let (tx_stop, rx_stop) = tokio::sync::oneshot::channel::<()>();

    let logger = logger.clone();
    let shutdown_task = async move {
        tokio::select! {
            _res0 = shutdown_cmd => {
                logger.info("Shutdown-1 command received, closing application");
            },

            _res1 = rx_stop => {
                logger.info("Manual shutdown signal received, closing application");
            }
        }
    };

    let (start_tx, start_rx) = tokio::sync::oneshot::channel::<()>();

    let combined_task = async move {
        start_rx
            .await
            .map_err(|_err| Report::msg("Failed to receive start signal"))?;

        tokio::select! {
            res0 = manager_task => {
                Err(Report::msg(format!("Blueprint Manager Closed Unexpectedly: {res0:?}")))
            },

            _ = shutdown_task => {
                Ok(())
            }
        }
    };

    let handle = tokio::spawn(combined_task);

    let handle = BlueprintManagerHandle {
        start_tx: Some(start_tx),
        shutdown_call: Some(tx_stop),
        running_task: handle,
        logger: logger_clone,
        sr25519_id: tangle_key,
        ecdsa_id: ecdsa_key,
        keystore_path: base_path,
    };

    Ok(handle)
}

/// * Query to get Vec<RpcServicesWithBlueprint>
/// * For each RpcServicesWithBlueprint, fetch the associated gadget binary (fetch/download)
///   -> If the services field is empty, just emit and log inside the executed binary "that states a new service instance got created by one of these blueprints"
///   -> If the services field is not empty, for each service in RpcServicesWithBlueprint.services, spawn the gadget binary, using params to set the job type to listen to (in terms of our old language, each spawned service represents a single "RoleType")
async fn handle_init(
    tangle_runtime: &TangleRuntimeClient,
    services_client: &ServicesClient<SubstrateConfig>,
    logger: &Logger,
    sub_account_id: &AccountId32,
    active_gadgets: &mut ActiveGadgets,
    gadget_config: &GadgetConfig,
    blueprint_manager_config: &BlueprintManagerConfig,
) -> color_eyre::Result<Vec<RpcServicesWithBlueprint>> {
    logger.info("Beginning initialization of Blueprint Manager");
    let (operator_subscribed_blueprints, init_event) =
        if let Some(event) = tangle_runtime.next_event().await {
            (
                get_blueprints(services_client, event.hash, sub_account_id.clone())
                    .await
                    .map_err(|err| Report::msg(format!("Failed to obtain blueprints: {err}")))?,
                event,
            )
        } else {
            return Err(Report::msg("Failed to get initial block hash"));
        };

    logger.info(format!(
        "Received {} initial blueprints this operator is registered to",
        operator_subscribed_blueprints.len()
    ));

    // Immediately poll, handling the initial state
    let poll_result =
        event_handler::check_blueprint_events(&init_event, logger, active_gadgets, sub_account_id)
            .await;

    event_handler::handle_tangle_event(
        &init_event,
        &operator_subscribed_blueprints,
        logger,
        gadget_config,
        blueprint_manager_config,
        active_gadgets,
        poll_result,
        services_client,
    )
    .await?;

    Ok(operator_subscribed_blueprints)
}

pub async fn get_blueprints<C: Config>(
    runtime: &ServicesClient<C>,
    block_hash: [u8; 32],
    account_id: AccountId32,
) -> color_eyre::Result<Vec<RpcServicesWithBlueprint>>
where
    BlockRef<<C as Config>::Hash>: From<BlockRef<H256>>,
{
    runtime
        .query_operator_blueprints(block_hash, account_id)
        .await
        .map_err(|err| msg_to_error(err.to_string()))
}
