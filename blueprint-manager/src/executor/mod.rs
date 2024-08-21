use crate::config::BlueprintManagerConfig;
use crate::gadget::ActiveGadgets;
use crate::sdk::entry::keystore_from_base_path;
use crate::sdk::utils;
use crate::sdk::utils::msg_to_error;
use crate::sdk::{Client, SendFuture};
use color_eyre::eyre::OptionExt;
use color_eyre::Report;
use gadget_common::config::DebugLogger;
use gadget_common::environments::GadgetEnvironment;
use gadget_common::subxt_signer::sr25519;
use gadget_common::tangle_runtime::AccountId32;
use gadget_io::standard::keystore::crypto;
use gadget_io::{GadgetConfig, KeystoreConfig, KeystoreContainer, SubstrateKeystore};
use sp_core::crypto::KeyTypeId;
use sp_core::{ecdsa, ByteArray, Pair, Public, H256};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tangle_environment::api::{RpcServicesWithBlueprint, ServicesClient};
use tangle_environment::gadget::SubxtConfig;
use tangle_environment::runtime::TangleRuntime;
use tangle_environment::TangleEnvironment;
use tangle_subxt::subxt::blocks::BlockRef;
use tangle_subxt::subxt::{Config, SubstrateConfig};
use tokio::task::JoinHandle;

pub(crate) mod event_handler;

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

pub struct BlueprintManagerHandle {
    shutdown_call: Option<tokio::sync::oneshot::Sender<()>>,
    start_tx: Option<tokio::sync::oneshot::Sender<()>>,
    process: JoinHandle<color_eyre::Result<()>>,
    logger: DebugLogger,
    keystore_container: KeystoreContainer,
    sr25519_id: sp_core::sr25519::Pair,
    ecdsa_id: ecdsa::Pair,
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
    pub fn sr25519_id(&self) -> sr25519::Keypair {
        let secret_key = self.expose_sr25519_secret();
        sr25519::Keypair::from_secret_key(secret_key).expect("Failed to create SR25519 keypair")
    }

    pub fn expose_sr25519_secret(&self) -> [u8; 32] {
        self.sr25519_id.as_ref().secret.to_bytes()[..32]
            .try_into()
            .unwrap()
    }

    /// Returns the ECDSA keypair for this blueprint manager
    pub fn ecdsa_id(&self) -> &ecdsa::Pair {
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

    /// Returns the keystore container for this blueprint manager
    pub fn key_store(&self) -> &KeystoreContainer {
        &self.keystore_container
    }

    /// Returns the logger for this blueprint manager
    pub fn logger(&self) -> &DebugLogger {
        &self.logger
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

        let result = futures::ready!(Pin::new(&mut this.process).poll(cx));

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
        "local"
    };

    let logger = DebugLogger {
        id: format!("Blueprint-Manager-{}", logger_id),
    };

    logger.info("Starting blueprint manager ... waiting for start signal ...");

    let logger_clone = logger.clone();

    let keystore_container = if blueprint_manager_config.test_mode {
        let seed = blueprint_manager_config
            .instance_id
            .clone()
            .expect("Should always exist for testing");
        logger.info(format!(
            "Running in test mode, auto-adding default keys for //{seed} ..."
        ));

        let keystore = KeystoreContainer::new(&KeystoreConfig::InMemory)?;
        let key_type = crypto::acco::KEY_TYPE;

        // Add both sr25519 and ECDSA account keys
        add_key_to_keystore::<sp_core::sr25519::Public>(&seed, key_type, &keystore)?;
        add_key_to_keystore::<ecdsa::Public>(&seed, key_type, &keystore)?;

        keystore
    } else {
        // For ordinary runs, we will default to the keystore path
        let keystore = keystore_from_base_path(
            &gadget_config.base_path,
            gadget_config.chain,
            gadget_config.keystore_password.clone(),
        );
        KeystoreContainer::new(&keystore)?
    };

    let (role_key, acco_key) = (
        keystore_container.ecdsa_key()?,
        keystore_container.sr25519_key()?,
    );
    let ecdsa_key = role_key.clone();

    let sub_account_id = AccountId32(acco_key.public().0);
    let subxt_config = SubxtConfig {
        endpoint: gadget_config.url.clone(),
    };

    let tangle_environment = TangleEnvironment::new(subxt_config, acco_key.clone(), logger.clone());

    let tangle_runtime = tangle_environment.setup_runtime().await?;
    let runtime = ServicesClient::new(logger.clone(), tangle_runtime.client());
    let mut active_gadgets = HashMap::new();

    let logger_manager = logger.clone();
    let manager_task = async move {
        // With the basics setup, we must now implement the main logic of the Blueprint Manager
        // Handle initialization logic
        // NOTE: The node running this code should be registered as an operator for the blueprints, otherwise, this
        // code will fail
        let mut operator_subscribed_blueprints = handle_init(
            &tangle_runtime,
            &runtime,
            &logger_manager,
            &sub_account_id,
            &mut active_gadgets,
            &gadget_config,
            &blueprint_manager_config,
        )
        .await?;

        // Now, run the main event loop
        // Listen to FinalityNotifications and poll for new/deleted services that correspond to the blueprints above
        while let Some(event) = tangle_runtime.next_event().await {
            let result = event_handler::check_blueprint_events(
                &event,
                &logger_manager,
                &mut active_gadgets,
                &sub_account_id,
            )
            .await;

            if result.needs_update {
                operator_subscribed_blueprints =
                    get_blueprints(&runtime, event.hash, sub_account_id.clone()).await?;
            }

            event_handler::handle_tangle_event(
                &event,
                &operator_subscribed_blueprints,
                &logger_manager,
                &gadget_config,
                &blueprint_manager_config,
                &mut active_gadgets,
                result,
                &runtime,
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
        process: handle,
        logger: logger_clone,
        sr25519_id: acco_key,
        ecdsa_id: ecdsa_key,
        keystore_container,
    };

    Ok(handle)
}

/// * Query to get Vec<RpcServicesWithBlueprint>
/// * For each RpcServicesWithBlueprint, fetch the associated gadget binary (fetch/download)
///   -> If the services field is empty, just emit and log inside the executed binary "that states a new service instance got created by one of these blueprints"
///   -> If the services field is not empty, for each service in RpcServicesWithBlueprint.services, spawn the gadget binary, using params to set the job type to listen to (in terms of our old language, each spawned service represents a single "RoleType")
async fn handle_init(
    tangle_runtime: &TangleRuntime,
    services_client: &ServicesClient<SubstrateConfig>,
    logger: &DebugLogger,
    sub_account_id: &AccountId32,
    active_gadgets: &mut ActiveGadgets,
    gadget_config: &GadgetConfig,
    blueprint_manager_config: &BlueprintManagerConfig,
) -> color_eyre::Result<Vec<RpcServicesWithBlueprint>> {
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

fn add_key_to_keystore<TPublic: Public>(
    seed: &str,
    key_type: KeyTypeId,
    keystore: &KeystoreContainer,
) -> color_eyre::Result<()> {
    let pub_key = get_from_seed::<TPublic>(seed).to_raw_vec();
    let keystore = keystore.keystore();
    if keystore
        .insert(key_type, &format!("//{seed}"), &pub_key)
        .is_err()
    {
        Err(Report::msg("Failed to insert key into keystore"))
    } else {
        Ok(())
    }
}

/// Helper function to generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{seed}"), None)
        .expect("static values are valid; qed")
        .public()
}
