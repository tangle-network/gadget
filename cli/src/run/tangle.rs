use alloy_signer_local::PrivateKeySigner;
use blueprint_manager::config::BlueprintManagerConfig;
use blueprint_manager::executor::run_blueprint_manager;
use color_eyre::eyre::{eyre, Result};
use gadget_config::GadgetConfiguration;
use gadget_crypto::tangle_pair_signer::TanglePairSigner;
use gadget_logging::info;
use sp_core::sr25519;
use std::path::PathBuf;
use tokio::signal;

#[derive(Clone)]
pub struct RunOpts {
    /// The HTTP RPC URL of the Tangle Network
    pub http_rpc_url: String,
    /// The WS RPC URL of the Tangle Network
    pub ws_rpc_url: String,
    /// The signer for Tangle operations
    pub signer: Option<TanglePairSigner<sr25519::Pair>>,
    /// The signer for EVM operations
    pub signer_evm: Option<PrivateKeySigner>,
    /// The blueprint ID to run
    pub blueprint_id: Option<u64>,
    /// The keystore path
    pub keystore_path: Option<String>,
    /// The data directory path
    pub data_dir: Option<PathBuf>,
}

/// Runs a blueprint using the blueprint manager
pub async fn run_blueprint(opts: RunOpts) -> Result<()> {
    let blueprint_id = opts
        .blueprint_id
        .ok_or_else(|| eyre!("Blueprint ID is required"))?;

    let mut gadget_config = GadgetConfiguration::default();
    gadget_config.http_rpc_endpoint = opts.http_rpc_url.clone();
    gadget_config.ws_rpc_endpoint = opts.ws_rpc_url.clone();

    if let Some(keystore_path) = opts.keystore_path {
        gadget_config.keystore_uri = keystore_path;
    }

    gadget_config.data_dir = opts.data_dir;

    let blueprint_manager_config = BlueprintManagerConfig {
        gadget_config: None,
        keystore_uri: gadget_config.keystore_uri.clone(),
        data_dir: gadget_config
            .data_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from("./data")),
        verbose: 2,
        pretty: true,
        instance_id: Some(format!("Blueprint-{}", blueprint_id)),
        test_mode: false,
    };

    info!(
        "Starting blueprint manager for blueprint ID: {}",
        blueprint_id
    );

    let shutdown_signal = async move {
        let _ = signal::ctrl_c().await;
        info!("Received shutdown signal, stopping blueprint manager");
    };

    let mut handle =
        run_blueprint_manager(blueprint_manager_config, gadget_config, shutdown_signal).await?;

    handle.start()?;

    handle.await?;

    info!("Blueprint manager has stopped");
    Ok(())
}
