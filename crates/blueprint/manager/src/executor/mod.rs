use crate::config::BlueprintManagerConfig;
use crate::error::Error;
use crate::error::Result;
use crate::gadget::ActiveGadgets;
use crate::sdk::entry::SendFuture;
use blueprint_sdk::runner::config::BlueprintEnvironment;
use color_eyre::Report;
use color_eyre::eyre::OptionExt;
use gadget_clients::tangle::EventsClient;
use gadget_clients::tangle::client::{TangleClient, TangleConfig};
use gadget_clients::tangle::services::{RpcServicesWithBlueprint, TangleServicesClient};
use gadget_crypto::sp_core::{SpEcdsa, SpSr25519};
use gadget_crypto::tangle_pair_signer::TanglePairSigner;
use gadget_keystore::backends::Backend;
use gadget_keystore::{Keystore, KeystoreConfig};
use gadget_logging::info;
use sp_core::{ecdsa, sr25519};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tangle_subxt::subxt::tx::Signer;
use tangle_subxt::subxt::utils::AccountId32;
use tokio::task::JoinHandle;

pub(crate) mod event_handler;

pub struct BlueprintManagerHandle {
    shutdown_call: Option<tokio::sync::oneshot::Sender<()>>,
    start_tx: Option<tokio::sync::oneshot::Sender<()>>,
    running_task: JoinHandle<color_eyre::Result<()>>,
    span: tracing::Span,
    sr25519_id: TanglePairSigner<sr25519::Pair>,
    ecdsa_id: TanglePairSigner<ecdsa::Pair>,
    keystore_uri: String,
}

impl BlueprintManagerHandle {
    /// Send a start signal to the blueprint manager
    ///
    /// # Errors
    ///
    /// * If the start signal fails to send
    /// * If the start signal has already been sent
    pub fn start(&mut self) -> color_eyre::Result<()> {
        let _span = self.span.enter();
        match self.start_tx.take() {
            Some(tx) => match tx.send(()) {
                Ok(()) => {
                    info!("Start signal sent to Blueprint Manager");
                    Ok(())
                }
                Err(()) => Err(Report::msg(
                    "Failed to send start signal to Blueprint Manager",
                )),
            },
            None => Err(Report::msg("Blueprint Manager Already Started")),
        }
    }

    /// Returns the SR25519 keypair for this blueprint manager
    #[must_use]
    pub fn sr25519_id(&self) -> &TanglePairSigner<sr25519::Pair> {
        &self.sr25519_id
    }

    /// Returns the ECDSA keypair for this blueprint manager
    #[must_use]
    pub fn ecdsa_id(&self) -> &TanglePairSigner<ecdsa::Pair> {
        &self.ecdsa_id
    }

    /// Shutdown the blueprint manager
    ///
    /// # Errors
    ///
    /// * If the shutdown signal fails to send
    /// * If the shutdown signal has already been sent
    pub fn shutdown(&mut self) -> color_eyre::Result<()> {
        self.shutdown_call
            .take()
            .map(|tx| tx.send(()))
            .ok_or_eyre("Shutdown already called")?
            .map_err(|()| Report::msg("Failed to send shutdown signal to Blueprint Manager"))
    }

    /// Returns the keystore URI for this blueprint manager
    #[must_use]
    pub fn keystore_uri(&self) -> &str {
        &self.keystore_uri
    }

    #[must_use]
    pub fn span(&self) -> &tracing::Span {
        &self.span
    }
}

/// Add default behavior for unintentional dropping of the `BlueprintManagerHandle`
/// This will ensure that the `BlueprintManagerHandle` is executed even if the handle
/// is dropped, which is similar behavior to the tokio `SpawnHandle`
impl Drop for BlueprintManagerHandle {
    fn drop(&mut self) {
        let _ = self.start();
    }
}

/// Implement the Future trait for the `BlueprintManagerHandle` to allow
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

/// Run the blueprint manager with the given configuration
///
/// # Arguments
///
/// * `blueprint_manager_config` - The configuration for the blueprint manager
/// * `gadget_config` - The configuration for the gadget
/// * `shutdown_cmd` - The shutdown command for the blueprint manager
///
/// # Returns
///
/// * A handle to the running blueprint manager
///
/// # Errors
///
/// * If the blueprint manager fails to start
///
/// # Panics
///
/// * If the SR25519 or ECDSA keypair cannot be found
#[allow(clippy::too_many_lines, clippy::used_underscore_binding)]
pub async fn run_blueprint_manager<F: SendFuture<'static, ()>>(
    blueprint_manager_config: BlueprintManagerConfig,
    env: BlueprintEnvironment,
    shutdown_cmd: F,
) -> color_eyre::Result<BlueprintManagerHandle> {
    let logger_id = if let Some(custom_id) = &blueprint_manager_config.instance_id {
        custom_id.as_str()
    } else {
        "Local"
    };

    let span = tracing::info_span!("Blueprint-Manager", id = logger_id);

    let _span = span.enter();
    info!("Starting blueprint manager ... waiting for start signal ...");

    let data_dir = &blueprint_manager_config.data_dir;
    if !data_dir.exists() {
        info!(
            "Data directory does not exist, creating it at `{}`",
            data_dir.display()
        );
        std::fs::create_dir_all(data_dir)?;
    }

    // TODO: Actual error handling
    let (tangle_key, ecdsa_key) = {
        let keystore = Keystore::new(KeystoreConfig::new().fs_root(&env.keystore_uri))?;
        let sr_key_pub = keystore.first_local::<SpSr25519>()?;
        let sr_pair = keystore.get_secret::<SpSr25519>(&sr_key_pub)?;
        let sr_key = TanglePairSigner::new(sr_pair.0);

        let ecdsa_key_pub = keystore.first_local::<SpEcdsa>()?;
        let ecdsa_pair = keystore.get_secret::<SpEcdsa>(&ecdsa_key_pub)?;
        let ecdsa_key = TanglePairSigner::new(ecdsa_pair.0);

        (sr_key, ecdsa_key)
    };

    let sub_account_id = tangle_key.account_id().clone();

    let mut active_gadgets = HashMap::new();

    let keystore_uri = env.keystore_uri.clone();

    let manager_task = async move {
        let tangle_client = TangleClient::new(env.clone()).await?;
        let services_client = tangle_client.services_client();

        // With the basics setup, we must now implement the main logic of the Blueprint Manager
        // Handle initialization logic
        // NOTE: The node running this code should be registered as an operator for the blueprints, otherwise, this
        // code will fail
        let mut operator_subscribed_blueprints = handle_init(
            &tangle_client,
            services_client,
            &sub_account_id,
            &mut active_gadgets,
            &env,
            &blueprint_manager_config,
        )
        .await?;

        // Now, run the main event loop
        // Listen to FinalityNotifications and poll for new/deleted services that correspond to the blueprints above
        while let Some(event) = tangle_client.next_event().await {
            let result = event_handler::check_blueprint_events(
                &event,
                &mut active_gadgets,
                &sub_account_id.clone(),
            );

            if result.needs_update {
                operator_subscribed_blueprints = services_client
                    .query_operator_blueprints(event.hash, sub_account_id.clone())
                    .await?;
            }

            event_handler::handle_tangle_event(
                &event,
                &operator_subscribed_blueprints,
                &env,
                &blueprint_manager_config,
                &mut active_gadgets,
                result,
                services_client,
            )
            .await?;
        }

        Err::<(), _>(Error::ClientDied)
    };

    let (tx_stop, rx_stop) = tokio::sync::oneshot::channel::<()>();

    let shutdown_task = async move {
        tokio::select! {
            _res0 = shutdown_cmd => {
                info!("Shutdown-1 command received, closing application");
            },

            _res1 = rx_stop => {
                info!("Manual shutdown signal received, closing application");
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

            () = shutdown_task => {
                Ok(())
            }
        }
    };

    drop(_span);
    let handle = tokio::spawn(combined_task);

    let handle = BlueprintManagerHandle {
        start_tx: Some(start_tx),
        shutdown_call: Some(tx_stop),
        running_task: handle,
        span,
        sr25519_id: tangle_key,
        ecdsa_id: ecdsa_key,
        keystore_uri,
    };

    Ok(handle)
}

/// * Query to get Vec<RpcServicesWithBlueprint>
/// * For each `RpcServicesWithBlueprint`, fetch the associated gadget binary (fetch/download)
///   -> If the services field is empty, just emit and log inside the executed binary "that states a new service instance got created by one of these blueprints"
///   -> If the services field is not empty, for each service in RpcServicesWithBlueprint.services, spawn the gadget binary, using params to set the job type to listen to (in terms of our old language, each spawned service represents a single "`RoleType`")
async fn handle_init(
    tangle_runtime: &TangleClient,
    services_client: &TangleServicesClient<TangleConfig>,
    sub_account_id: &AccountId32,
    active_gadgets: &mut ActiveGadgets,
    blueprint_env: &BlueprintEnvironment,
    blueprint_manager_config: &BlueprintManagerConfig,
) -> Result<Vec<RpcServicesWithBlueprint>> {
    info!("Beginning initialization of Blueprint Manager");

    let Some(init_event) = tangle_runtime.next_event().await else {
        return Err(Error::InitialBlock);
    };

    let operator_subscribed_blueprints = services_client
        .query_operator_blueprints(init_event.hash, sub_account_id.clone())
        .await?;

    info!(
        "Received {} initial blueprints this operator is registered to",
        operator_subscribed_blueprints.len()
    );

    // Immediately poll, handling the initial state
    let poll_result =
        event_handler::check_blueprint_events(&init_event, active_gadgets, sub_account_id);

    event_handler::handle_tangle_event(
        &init_event,
        &operator_subscribed_blueprints,
        blueprint_env,
        blueprint_manager_config,
        active_gadgets,
        poll_result,
        services_client,
    )
    .await?;

    Ok(operator_subscribed_blueprints)
}
