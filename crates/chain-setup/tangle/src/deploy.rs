use dialoguer::console::style;
use gadget_std::{env, rand};
use alloy_provider::network::TransactionBuilder;
use alloy_provider::{Provider, WsConnect};
use alloy_rpc_types_eth::TransactionRequest;
use alloy_signer_local::PrivateKeySigner;
use color_eyre::eyre::{self, Context, ContextCompat, Result};
use blueprint_tangle_extra::metadata::types::blueprint::BlueprintServiceManager;
use gadget_crypto::tangle_pair_signer::TanglePairSigner;
use gadget_std::fmt::Debug;
use gadget_std::path::PathBuf;
use serde_json::Value;
use subxt::tx::Signer;
use tangle_subxt::subxt;
use tangle_subxt::tangle_testnet_runtime::api as TangleApi;
use tangle_subxt::tangle_testnet_runtime::api::services::calls::types;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::service::ServiceBlueprint;
use gadget_chain_setup_common::signer::{load_evm_signer_from_env, load_signer_from_env};
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

#[derive(Clone)]
pub struct Opts {
    /// The name of the package to deploy (if the workspace has multiple packages)
    pub pkg_name: Option<String>,
    /// The HTTP RPC URL of the Tangle Network
    pub http_rpc_url: String,
    /// The WS RPC URL of the Tangle Network
    pub ws_rpc_url: String,
    /// The path to the manifest file
    pub manifest_path: gadget_std::path::PathBuf,
    /// The signer for deploying the blueprint
    pub signer: Option<TanglePairSigner<sp_core::sr25519::Pair>>,
    /// The signer for deploying the smart contract
    pub signer_evm: Option<PrivateKeySigner>,
}

impl Debug for Opts {
    fn fmt(&self, f: &mut gadget_std::fmt::Formatter<'_>) -> gadget_std::fmt::Result {
        f.debug_struct("Opts")
            .field("pkg_name", &self.pkg_name)
            .field("http_rpc_url", &self.http_rpc_url)
            .field("ws_rpc_url", &self.ws_rpc_url)
            .field("manifest_path", &self.manifest_path)
            .finish_non_exhaustive()
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Unsupported blueprint manager kind")]
    UnsupportedBlueprintManager,
    #[error("Contract not found at `{0}`, check the manager in your `Cargo.toml`!")]
    ContractNotFound(PathBuf),
    #[error("Failed to deserialize contract `{0}`: {1}")]
    DeserializeContract(String, serde_json::Error),
    #[error("The source at index {0} does not have a valid fetcher")]
    MissingFetcher(usize),
    #[error("No matching packages found in the workspace")]
    NoPackageFound,
    #[error("The workspace has multiple packages, please specify the package to deploy")]
    ManyPackages,

    #[error("{0}")]
    Io(#[from] std::io::Error),
}

async fn generate_service_blueprint<P: Into<PathBuf>, T: AsRef<str>>(
    manifest_metadata_path: P,
    pkg_name: Option<&String>,
    rpc_url: T,
    signer_evm: Option<PrivateKeySigner>,
) -> Result<types::create_blueprint::Blueprint> {
    let manifest_path = manifest_metadata_path.into();
    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(manifest_path)
        .no_deps()
        .exec()
        .context("Getting Metadata about the workspace")?;

    let package = find_package(&metadata, pkg_name)?.clone();

    let mut blueprint = load_blueprint_metadata(&package)?;
    build_contracts_if_needed(&package, &blueprint).context("Building contracts")?;
    deploy_contracts_to_tangle(rpc_url.as_ref(), &package, &mut blueprint, signer_evm).await?;
    bake_blueprint(&blueprint)
}

/// Deploy a blueprint to the Tangle Network
///
/// # Errors
///
/// * Any blueprint metadata is malformed
/// * If `signer` is not provided, see [`load_signer_from_env()`]
/// * `ws_rpc_url` is invalid
/// * No `BlueprintCreated` event found under `signer`
///
/// [`load_signer_from_env()`]: crate::signer::load_signer_from_env
pub async fn deploy_to_tangle(
    Opts {
        pkg_name,
        http_rpc_url: _,
        ws_rpc_url,
        manifest_path,
        signer,
        signer_evm,
    }: Opts,
) -> Result<u64> {
    // Create a progress bar to track deployment
    let progress_bar = ProgressBar::new(100);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}% {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    // Helper function to update progress
    let update_progress = |percent: u64, message: &str| {
        progress_bar.set_position(percent);
        progress_bar.set_message(message.to_string());
    };

    // Start the deployment process
    update_progress(0, "Generating blueprint");

    // Create a flag to signal the thread to stop
    let should_stop = Arc::new(AtomicBool::new(false));
    let should_stop_clone = should_stop.clone();

    // Create a background thread to update the progress bar during the long-running operation
    let progress_clone = progress_bar.clone();
    let _progress_handle = std::thread::spawn(move || {
        let mut current = 0;
        let target = 80;
        let increment = rand::random::<u64>() % 2 + 1;
        let sleep_duration = std::time::Duration::from_millis(1000);

        while current < target && !should_stop_clone.load(Ordering::Relaxed) {
            std::thread::sleep(sleep_duration);
            current += increment;
            progress_clone.set_position(current);
            progress_clone.set_message(format!("Generating blueprint... ({}%)", current));
        }
    });

    // Load the manifest file into cargo metadata
    let blueprint =
        generate_service_blueprint(&manifest_path, pkg_name.as_ref(), &ws_rpc_url, signer_evm)
            .await?;

    // Signal the thread to stop
    should_stop.store(true, Ordering::Relaxed);

    // Blueprint generation is complete, update to 80% if not already there
    if progress_bar.position() < 80 {
        update_progress(80, "Blueprint generated");
    } else {
        update_progress(progress_bar.position(), "Blueprint generated");
    }

    // Give the thread a moment to notice the stop signal
    std::thread::sleep(std::time::Duration::from_millis(100));

    let signer = if let Some(signer) = signer {
        signer
    } else {
        load_signer_from_env()?
    };

    let my_account_id = signer.account_id();
    update_progress(85, "Connecting");
    let client = subxt::OnlineClient::from_url(ws_rpc_url.clone()).await?;
    tracing::info!("Connected to Tangle Network at: {}", ws_rpc_url);

    update_progress(90, "Creating blueprint transaction");
    let create_blueprint_tx = TangleApi::tx().services().create_blueprint(blueprint);
    tracing::info!("Created blueprint...");

    update_progress(93, "Signing and submitting transaction");
    let progress = client
        .tx()
        .sign_and_submit_then_watch_default(&create_blueprint_tx, &signer)
        .await?;

    update_progress(95, "Waiting for transaction confirmation");
    let result = if cfg!(test) {
        use blueprint_tangle_extra::util::TxProgressExt;
        progress.wait_for_in_block_success().await?
    } else {
        tracing::debug!("Waiting for the transaction to be finalized...");
        let result = progress.wait_for_finalized_success().await?;
        tracing::debug!("Transaction finalized...");
        result
    };

    update_progress(98, "Verifying blueprint creation");
    let event = result
        .find::<TangleApi::services::events::BlueprintCreated>()
        .flatten()
        .find(|e| e.owner.0 == my_account_id.0)
        .context("Finding the `BlueprintCreated` event")
        .map_err(|e| {
            eyre::eyre!(
                "Trying to find the `BlueprintCreated` event with your account Id: {:?}",
                e
            )
        })?;

    update_progress(100, "Deployment complete");
    progress_bar.finish_with_message("Blueprint deployed successfully!");

    tracing::info!(
        "Blueprint #{} created successfully by {} with extrinsic hash: {}",
        event.blueprint_id,
        event.owner,
        result.extrinsic_hash(),
    );

    Ok(event.blueprint_id)
}

/// Gets either `CARGO_WORKSPACE_DIR` for Tangle blueprints, or the workspace root directory
/// identified by finding a Cargo.toml with a [workspace] section. If no workspace is found,
/// falls back to the package manifest directory.
fn workspace_or_package_manifest_path(package: &cargo_metadata::Package) -> PathBuf {
    env::var("CARGO_WORKSPACE_DIR").map_or_else(|_| find_workspace_root(package), PathBuf::from)
}

/// Finds the workspace root directory by walking up the directory tree and looking for a
/// Cargo.toml with a [workspace] section. If no workspace root is found, returns the package directory.
fn find_workspace_root(package: &cargo_metadata::Package) -> PathBuf {
    let package_dir = package
        .manifest_path
        .parent()
        .unwrap()
        .as_std_path()
        .to_path_buf();
    let mut current_dir = package_dir.clone();
    let mut workspace_root = package_dir;

    // Walk up the directory tree looking for a workspace root
    while let Some(parent) = current_dir.parent() {
        let potential_cargo_toml = parent.join("Cargo.toml");
        tracing::debug!(
            "Looking for Cargo.toml at: {}",
            potential_cargo_toml.display()
        );
        if potential_cargo_toml.exists() {
            tracing::debug!("Found Cargo.toml");
            // Check if this Cargo.toml has a [workspace] section
            if let Ok(content) = std::fs::read_to_string(&potential_cargo_toml) {
                if content.contains("[workspace]") {
                    tracing::debug!(
                        "Found [workspace] section, using this directory as workspace root"
                    );
                    workspace_root = parent.to_path_buf();
                    break;
                }
            } else {
            }
        }
        current_dir = parent.to_path_buf();

        // Stop at the filesystem root
        if current_dir.parent().is_none() {
            break;
        }
    }

    tracing::debug!("Identified workspace root: {:?}", workspace_root);
    workspace_root
}

fn load_blueprint_metadata(
    package: &cargo_metadata::Package,
) -> Result<blueprint_tangle_extra::metadata::types::blueprint::ServiceBlueprint<'static>> {
    // Find the workspace root
    let workspace_root = find_workspace_root(package);
    let package_dir = package
        .manifest_path
        .parent()
        .unwrap()
        .as_std_path()
        .to_path_buf();

    // First check in the workspace root directory
    let mut blueprint_json_path = workspace_root.join("blueprint.json");

    // If not found in workspace root, check in the binary's directory
    if !blueprint_json_path.exists() {
        blueprint_json_path = package_dir.join("blueprint.json");
    }

    if !blueprint_json_path.exists() {
        tracing::warn!(
            "Could not find blueprint.json in workspace root or binary directory; running `cargo build`..."
        );
        tracing::debug!(
            "Looked for blueprint.json at workspace root: {:?}",
            workspace_root.join("blueprint.json")
        );
        tracing::debug!(
            "Looked for blueprint.json at package dir: {:?}",
            package_dir.join("blueprint.json")
        );

        // Need to run cargo build. We don't know the package name, so unfortunately this will
        // build the entire workspace.
        escargot::CargoBuild::new()
            .manifest_path(workspace_root.join("Cargo.toml"))
            .run()
            .context("Failed to build the package")?;

        // After building, check both locations again
        blueprint_json_path = workspace_root.join("blueprint.json");

        if !blueprint_json_path.exists() {
            blueprint_json_path = package_dir.join("blueprint.json");

            if !blueprint_json_path.exists() {
                return Err(eyre::eyre!(
                    "Could not find blueprint.json after building. Checked:\n\
                     - Workspace root: {:?}\n\
                     - Package directory: {:?}",
                    workspace_root.join("blueprint.json"),
                    package_dir.join("blueprint.json")
                ));
            }
        }
    }

    tracing::debug!("Found blueprint.json at: {:?}", blueprint_json_path);

    // should have the blueprint.json
    let blueprint_json = std::fs::read_to_string(&blueprint_json_path).context(format!(
        "Reading blueprint.json file at {:?}",
        blueprint_json_path
    ))?;
    let blueprint = serde_json::from_str(&blueprint_json)?;
    Ok(blueprint)
}

async fn deploy_contracts_to_tangle(
    rpc_url: &str,
    package: &cargo_metadata::Package,
    blueprint: &mut blueprint_tangle_extra::metadata::types::blueprint::ServiceBlueprint<'static>,
    signer_evm: Option<PrivateKeySigner>,
) -> Result<()> {
    enum ContractKind {
        Manager,
    }
    let contract_paths = match blueprint.manager {
        BlueprintServiceManager::Evm(ref path) => vec![(ContractKind::Manager, path)],
        _ => return Err(Error::UnsupportedBlueprintManager.into()),
    };

    let abs_contract_paths: Vec<_> = contract_paths
        .into_iter()
        .map(|(kind, path)| (kind, resolve_path_relative_to_package(package, path)))
        .collect();

    let mut contracts_raw = Vec::new();
    for (kind, path) in abs_contract_paths {
        if !path.exists() {
            return Err(Error::ContractNotFound(path).into());
        }

        let content = std::fs::read_to_string(&path)?;
        contracts_raw.push((
            kind,
            path.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            content,
        ));
    }

    let mut contracts = Vec::new();
    for (kind, contract_name, json) in contracts_raw {
        let contract = match serde_json::from_str::<alloy_json_abi::ContractObject>(&json) {
            Ok(contract) => contract,
            Err(e) => return Err(Error::DeserializeContract(contract_name, e).into()),
        };

        contracts.push((kind, contract_name, contract));
    }

    if contracts.is_empty() {
        return Ok(());
    }

    let signer = if let Some(signer) = signer_evm {
        signer
    } else {
        load_evm_signer_from_env()?
    };

    let wallet = alloy_provider::network::EthereumWallet::from(signer);
    assert!(rpc_url.starts_with("ws:"));

    let provider = alloy_provider::ProviderBuilder::new()
        .wallet(wallet)
        .on_ws(WsConnect::new(rpc_url))
        .await?;

    let chain_id = provider.get_chain_id().await?;
    tracing::debug!("Chain ID: {chain_id}");

    for (kind, name, contract) in contracts {
        tracing::info!("Deploying contract: {name} ...");
        let Some(bytecode) = contract.bytecode.clone() else {
            tracing::warn!("Contract {name} does not have deployed bytecode! Skipping ...");
            continue;
        };

        let tx = TransactionRequest::default().with_deploy_code(bytecode);
        // Deploy the contract.
        let receipt = provider.send_transaction(tx).await?.get_receipt().await?;
        // Check the receipt status.
        if receipt.status() {
            let contract_address =
                alloy_network::ReceiptResponse::contract_address(&receipt).unwrap();
            tracing::info!("Contract {name} deployed at: {contract_address}");
            println!(
                "   {}",
                style(format!("Contract {name} deployed at: {contract_address}")).yellow()
            );

            match kind {
                ContractKind::Manager => {
                    blueprint.manager = BlueprintServiceManager::Evm(contract_address.to_string());
                }
            }
        } else {
            tracing::error!("Contract {name} deployment failed!");
            tracing::debug!("Receipt: {receipt:#?}");
        }
    }
    Ok(())
}

/// Checks if the contracts need to be built and builds them if needed.
fn build_contracts_if_needed(
    package: &cargo_metadata::Package,
    blueprint: &blueprint_tangle_extra::metadata::types::blueprint::ServiceBlueprint<'static>,
) -> Result<()> {
    let pathes_to_check = match blueprint.manager {
        BlueprintServiceManager::Evm(ref path) => vec![path],
        _ => return Err(Error::UnsupportedBlueprintManager.into()),
    };

    tracing::debug!("Checking for contracts to build: {pathes_to_check:?}");

    let abs_pathes_to_check: Vec<_> = pathes_to_check
        .into_iter()
        .map(|path| resolve_path_relative_to_package(package, path))
        .collect();

    tracing::debug!("Absolute paths to check: {abs_pathes_to_check:?}");

    let needs_build = abs_pathes_to_check.iter().any(|path| !path.exists());
    if !needs_build {
        tracing::debug!("All contracts are already built");
        return Ok(());
    }

    tracing::debug!("Contracts need to be built");

    // Find the workspace root
    let workspace_root = find_workspace_root(package);
    let package_dir = package
        .manifest_path
        .parent()
        .unwrap()
        .as_std_path()
        .to_path_buf();

    tracing::debug!("Workspace root directory: {workspace_root:?}");

    // Look for contracts directory in the workspace root
    let mut contracts_dir = workspace_root.join("contracts");
    if !contracts_dir.exists() {
        tracing::debug!("Contracts directory not found in workspace root: {contracts_dir:?}");

        // Fall back to package directory if not found in workspace root
        let package_contracts_dir = package_dir.join("contracts");
        if !package_contracts_dir.exists() {
            tracing::debug!(
                "Contracts directory not found in package directory: {package_contracts_dir:?}"
            );
            return Err(Error::ContractNotFound(contracts_dir).into());
        }

        tracing::debug!("Using contracts directory from package: {package_contracts_dir:?}");
        contracts_dir = package_contracts_dir;
    }

    tracing::debug!("Contracts directory: {contracts_dir:?}");

    let foundry = crate::foundry::FoundryToolchain::new();
    foundry.check_installed_or_exit();

    // Change to workspace root directory before building
    tracing::debug!("Changing to workspace root directory: {workspace_root:?}");
    std::env::set_current_dir(workspace_root)?;
    foundry.forge.install_dependencies()?;
    foundry.forge.build()?;

    // Verify the build succeeded
    for path in &abs_pathes_to_check {
        if !path.exists() {
            return Err(Error::ContractNotFound(path.clone()).into());
        }
    }

    Ok(())
}

/// Converts the `ServiceBlueprint` to a format that can be sent to the Tangle Network.
fn bake_blueprint(
    blueprint: &blueprint_tangle_extra::metadata::types::blueprint::ServiceBlueprint<'static>,
) -> Result<ServiceBlueprint> {
    let mut blueprint_json = serde_json::to_value(blueprint)?;
    convert_to_bytes_or_null(&mut blueprint_json["metadata"]["name"]);
    convert_to_bytes_or_null(&mut blueprint_json["metadata"]["description"]);
    convert_to_bytes_or_null(&mut blueprint_json["metadata"]["author"]);
    convert_to_bytes_or_null(&mut blueprint_json["metadata"]["license"]);
    convert_to_bytes_or_null(&mut blueprint_json["metadata"]["website"]);
    convert_to_bytes_or_null(&mut blueprint_json["metadata"]["code_repository"]);
    convert_to_bytes_or_null(&mut blueprint_json["metadata"]["category"]);

    // Set the Hooks to be empty to be compatible with the old blueprint format.
    // This is because the new blueprint format has manager field instead of different hooks.
    blueprint_json["registration_hook"] = serde_json::json!("None");
    blueprint_json["request_hook"] = serde_json::json!("None");
    for job in blueprint_json["jobs"].as_array_mut().unwrap() {
        convert_to_bytes_or_null(&mut job["metadata"]["name"]);
        convert_to_bytes_or_null(&mut job["metadata"]["description"]);
        // Set an empty verifier to be compatible with the old blueprint format.
        job["verifier"] = serde_json::json!("None");
    }

    // Retrieves the Gadget information from the blueprint.json file.
    // From this data, we find the sources of the Gadget.
    let (_, gadget) = blueprint_json["gadget"]
        .as_object_mut()
        .expect("Bad gadget value")
        .iter_mut()
        .next()
        .expect("Should be at least one gadget");
    let sources = gadget["sources"].as_array_mut().expect("Should be a list");

    // The source includes where to fetch the Blueprint, the name of the blueprint, and
    // the name of the binary. From this data, we create the blueprint's [`ServiceBlueprint`]
    for (idx, source) in sources.iter_mut().enumerate() {
        let Some(fetchers) = source["fetcher"].as_object_mut() else {
            return Err(Error::MissingFetcher(idx).into());
        };

        let fetcher_fields = fetchers
            .iter_mut()
            .next()
            .expect("Should be at least one fetcher")
            .1;
        for (_key, value) in fetcher_fields
            .as_object_mut()
            .expect("Fetcher should be a map")
        {
            if value.is_array() {
                let xs = value.as_array_mut().expect("Value should be an array");
                for x in xs {
                    if x.is_object() {
                        convert_to_bytes_or_null(&mut x["name"]);
                    }
                }
            } else {
                convert_to_bytes_or_null(value);
            }
        }
    }

    // TODO
    blueprint_json["supported_membership_models"] =
        Value::Array(vec![Value::String(String::from("Fixed"))]);

    let blueprint = serde_json::from_value(blueprint_json)?;
    Ok(blueprint)
}

/// Recursively converts a JSON string (or array of JSON strings) to bytes.
///
/// Empty strings are converted to nulls.
fn convert_to_bytes_or_null(v: &mut serde_json::Value) {
    if let serde_json::Value::String(s) = v {
        *v = serde_json::Value::Array(s.bytes().map(serde_json::Value::from).collect());
        return;
    }

    if let serde_json::Value::Array(vals) = v {
        for val in vals {
            convert_to_bytes_or_null(val);
        }
    }
}

/// Resolves a path relative to the package manifest.
fn resolve_path_relative_to_package(
    package: &cargo_metadata::Package,
    path: &str,
) -> std::path::PathBuf {
    if path.starts_with('/') {
        std::path::PathBuf::from(path)
    } else {
        workspace_or_package_manifest_path(package).join(path)
    }
}

/// Finds a package in the workspace to deploy.
fn find_package<'m>(
    metadata: &'m cargo_metadata::Metadata,
    pkg_name: Option<&String>,
) -> Result<&'m cargo_metadata::Package, eyre::Error> {
    match metadata.workspace_members.len() {
        0 => Err(Error::NoPackageFound.into()),
        1 => metadata
            .packages
            .iter()
            .find(|p| p.id == metadata.workspace_members[0])
            .ok_or(Error::NoPackageFound.into()),
        _more_than_one if pkg_name.is_some() => metadata
            .packages
            .iter()
            .find(|p| pkg_name.is_some_and(|v| &p.name == v))
            .ok_or(Error::NoPackageFound.into()),
        _otherwise => {
            // Find all binary packages in the workspace
            let bin_packages: Vec<&cargo_metadata::Package> = metadata
                .packages
                .iter()
                .filter(|p| {
                    p.targets
                        .iter()
                        .any(|t| t.kind.contains(&"bin".to_string()))
                })
                .collect();

            match bin_packages.len() {
                0 => {
                    eprintln!("No binary packages found in the workspace.");
                    Err(Error::NoPackageFound.into())
                }
                1 => {
                    // If there's only one binary package, use it automatically
                    tracing::info!(
                        "Automatically selecting the only binary package: {}",
                        bin_packages[0].name
                    );
                    Ok(bin_packages[0])
                }
                _ => {
                    // If there are multiple binary packages, prompt for selection
                    eprintln!(
                        "Multiple binary packages found. Please specify the package to deploy:"
                    );
                    for package in &bin_packages {
                        eprintln!("Found: {}", package.name);
                    }
                    eprintln!();
                    Err(Error::ManyPackages.into())
                }
            }
        }
    }
}
