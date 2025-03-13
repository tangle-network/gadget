use alloy_signer_local::PrivateKeySigner;
use blueprint_manager::config::BlueprintManagerConfig;
use blueprint_manager::executor::run_blueprint_manager;
use blueprint_runner::config::BlueprintEnvironment;
use color_eyre::eyre::{Result, eyre};
use dialoguer::console::style;
use gadget_crypto::tangle_pair_signer::TanglePairSigner;
use indicatif::{ProgressBar, ProgressStyle};
use sp_core::sr25519;
use std::path::PathBuf;
use std::time::Duration;
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
///
/// # Arguments
///
/// * `opts` - Options for running the blueprint
///
/// # Errors
///
/// Returns an error if:
/// * Blueprint ID is not provided
/// * Failed to create or configure the blueprint manager
/// * Failed to run the blueprint
pub async fn run_blueprint(opts: RunOpts) -> Result<()> {
    let blueprint_id = opts
        .blueprint_id
        .ok_or_else(|| eyre!("Blueprint ID is required"))?;

    let mut gadget_config = BlueprintEnvironment::default();
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

    println!("{}", style(format!("Starting blueprint manager for blueprint ID: {}", blueprint_id)).cyan().bold());

    let shutdown_signal = async move {
        let _ = signal::ctrl_c().await;
        println!("{}", style("Received shutdown signal, stopping blueprint manager").yellow().bold());
    };

    println!("{}", style("Preparing Blueprint to run, this may take a few minutes...").cyan());
    
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
            .template("{spinner:.blue} {msg}")
            .unwrap()
    );
    pb.set_message("Initializing Blueprint");
    pb.enable_steady_tick(Duration::from_millis(100));
    
    let mut handle = run_blueprint_manager(blueprint_manager_config, gadget_config, shutdown_signal).await?;
    
    pb.finish_with_message("Blueprint initialized successfully!");

    println!("{}", style("Starting blueprint execution...").green().bold());
    handle.start()?;

    println!("{}", style("Blueprint is running. Press Ctrl+C to stop.").cyan());
    handle.await?;

    println!("{}", style("Blueprint manager has stopped").green());
    Ok(())
}
