use alloy_provider::network::TransactionBuilder;
use alloy_provider::Provider;
use color_eyre::eyre::{self, Context, ContextCompat, OptionExt, Result};
use gadget_blueprint_proc_macro_core::{
    JobResultVerifier, ServiceBlueprint, ServiceRegistrationHook, ServiceRequestHook,
};
use std::path::PathBuf;
use tangle_subxt::subxt::{self, PolkadotConfig};
use tangle_subxt::tangle_testnet_runtime::api as TangleApi;
use tangle_subxt::tangle_testnet_runtime::api::services::calls::types;

#[derive(Debug, Clone)]
pub struct Opts {
    /// The name of the package to deploy (if the workspace has multiple packages)
    pub pkg_name: Option<String>,
    /// The RPC URL of the Tangle Network
    pub rpc_url: String,
    /// The path to the manifest file
    pub manifest_path: std::path::PathBuf,
}

pub async fn generate_service_blueprint<P: Into<PathBuf>, T: AsRef<str>>(
    manifest_metadata_path: P,
    pkg_name: Option<&String>,
    rpc_url: T,
) -> Result<types::create_blueprint::Blueprint> {
    let manifest_path = manifest_metadata_path.into();
    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(manifest_path)
        .no_deps()
        .exec()
        .context("Getting Metadata about the workspace")?;

    let package = find_package(&metadata, pkg_name)?;
    let mut blueprint = load_blueprint_metadata(package)?;

    build_contracts_if_needed(package, &blueprint).context("Building contracts")?;
    deploy_contracts_to_tangle(rpc_url.as_ref(), package, &mut blueprint).await?;

    bake_blueprint(blueprint)
}

pub async fn deploy_to_tangle(
    Opts {
        pkg_name,
        rpc_url,
        manifest_path,
    }: Opts,
) -> Result<()> {
    // Load the manifest file into cargo metadata
    let blueprint = generate_service_blueprint(&manifest_path, pkg_name.as_ref(), &rpc_url).await?;

    let signer = crate::signer::load_signer_from_env()?;
    let my_account_id = signer.public_key().to_account_id();
    let client = subxt::OnlineClient::<PolkadotConfig>::from_url(rpc_url).await?;

    let create_blueprint_tx = TangleApi::tx().services().create_blueprint(blueprint);

    let progress = client
        .tx()
        .sign_and_submit_then_watch_default(&create_blueprint_tx, &signer)
        .await?;
    let result = progress.wait_for_finalized_success().await?;
    let event = result
        .find::<TangleApi::services::events::BlueprintCreated>()
        .flatten()
        .find(|e| e.owner == my_account_id)
        .context("Finding the `BlueprintCreated` event")
        .map_err(|e| {
            eyre::eyre!(
                "Trying to find the `BlueprintCreated` event with your account Id: {:?}",
                e
            )
        })?;
    println!(
        "Blueprint #{} created successfully by {} with extrinsic hash: {}",
        event.blueprint_id,
        event.owner,
        result.extrinsic_hash(),
    );
    Ok(())
}

pub fn load_blueprint_metadata(package: &cargo_metadata::Package) -> Result<ServiceBlueprint> {
    let blueprint_json_path = package
        .manifest_path
        .parent()
        .map(|p| p.join("blueprint.json"))
        .unwrap();

    if !blueprint_json_path.exists() {
        eprintln!("Could not find blueprint.json; running `cargo build`...");
        // Need to run cargo build for the current package.
        escargot::CargoBuild::new()
            .manifest_path(&package.manifest_path)
            .package(&package.name)
            .run()
            .context("Failed to build the package")?;
    }
    // should have the blueprnt.json
    let blueprint_json =
        std::fs::read_to_string(blueprint_json_path).context("Reading blueprint.json file")?;
    let blueprint = serde_json::from_str::<ServiceBlueprint<'_>>(&blueprint_json)?;
    Ok(blueprint)
}

async fn deploy_contracts_to_tangle(
    rpc_url: &str,
    package: &cargo_metadata::Package,
    blueprint: &mut ServiceBlueprint<'_>,
) -> Result<()> {
    enum ContractKind {
        RegistrationHook,
        RequestHook,
        JobVerifier(usize),
    }
    let rpc_url = rpc_url.replace("ws", "http").replace("wss", "https");
    let mut contract_paths = Vec::new();
    match blueprint.registration_hook {
        ServiceRegistrationHook::None => {}
        ServiceRegistrationHook::Evm(ref path) => {
            contract_paths.push((ContractKind::RegistrationHook, path))
        }
    };

    match blueprint.request_hook {
        ServiceRequestHook::None => {}
        ServiceRequestHook::Evm(ref path) => contract_paths.push((ContractKind::RequestHook, path)),
    };

    for (id, job) in blueprint.jobs.iter().enumerate() {
        match job.verifier {
            JobResultVerifier::None => {}
            JobResultVerifier::Evm(ref path) => {
                contract_paths.push((ContractKind::JobVerifier(id), path))
            }
        };
    }

    let abs_contract_paths: Vec<_> = contract_paths
        .into_iter()
        .map(|(kind, path)| (kind, resolve_path_relative_to_package(package, path)))
        .collect();

    let contracts = abs_contract_paths
        .iter()
        .flat_map(|(kind, path)| {
            std::fs::read_to_string(path).map(|content| {
                (
                    kind,
                    path.file_stem().unwrap_or_default().to_string_lossy(),
                    content,
                )
            })
        })
        .flat_map(|(kind, contract_name, json)| {
            serde_json::from_str::<alloy_json_abi::ContractObject>(&json)
                .map(|contract| (kind, contract_name, contract))
        })
        .collect::<Vec<_>>();

    if contracts.is_empty() {
        return Ok(());
    }

    let signer = crate::signer::load_evm_signer_from_env()?;
    let wallet = alloy_provider::network::EthereumWallet::from(signer);
    let provider = alloy_provider::ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_http(rpc_url.parse()?);

    let chain_id = provider.get_chain_id().await?;
    eprintln!("Chain ID: {chain_id}");

    for (kind, name, contract) in contracts {
        eprintln!("Deploying contract: {name} ...");
        let Some(bytecode) = contract.bytecode.clone() else {
            eprintln!("Contract {name} does not have deployed bytecode! Skipping ...");
            continue;
        };
        let tx = alloy_rpc_types::TransactionRequest::default().with_deploy_code(bytecode);
        // Deploy the contract.
        let receipt = provider.send_transaction(tx).await?.get_receipt().await?;
        // Check the receipt status.
        if receipt.status() {
            let contract_address =
                alloy_network::ReceiptResponse::contract_address(&receipt).unwrap();
            eprintln!("Contract {name} deployed at: {contract_address}");
            match kind {
                ContractKind::RegistrationHook => {
                    blueprint.registration_hook =
                        ServiceRegistrationHook::Evm(contract_address.to_string());
                }
                ContractKind::RequestHook => {
                    blueprint.request_hook = ServiceRequestHook::Evm(contract_address.to_string());
                }
                ContractKind::JobVerifier(id) => {
                    blueprint.jobs[*id].verifier =
                        JobResultVerifier::Evm(contract_address.to_string());
                }
            }
        } else {
            eprintln!("Contract {name} deployment failed!");
            eprintln!("Receipt: {receipt:#?}");
        }
    }
    Ok(())
}

/// Checks if the contracts need to be built and builds them if needed.
fn build_contracts_if_needed(
    package: &cargo_metadata::Package,
    blueprint: &ServiceBlueprint,
) -> Result<()> {
    let mut pathes_to_check = Vec::new();

    match blueprint.registration_hook {
        ServiceRegistrationHook::None => {}
        ServiceRegistrationHook::Evm(ref path) => pathes_to_check.push(path),
    };

    match blueprint.request_hook {
        ServiceRequestHook::None => {}
        ServiceRequestHook::Evm(ref path) => pathes_to_check.push(path),
    };

    for job in blueprint.jobs.iter() {
        match job.verifier {
            JobResultVerifier::None => {}
            JobResultVerifier::Evm(ref path) => pathes_to_check.push(path),
        };
    }

    let abs_pathes_to_check: Vec<_> = pathes_to_check
        .into_iter()
        .map(|path| resolve_path_relative_to_package(package, path))
        .collect();

    let needs_build = abs_pathes_to_check.iter().any(|path| !path.exists());

    if needs_build {
        let mut foundry = crate::foundry::FoundryToolchain::new();
        foundry.check_installed_or_exit();
        foundry.forge.build()?;
    }

    Ok(())
}

/// Converts the ServiceBlueprint to a format that can be sent to the Tangle Network.
fn bake_blueprint(
    blueprint: ServiceBlueprint,
) -> Result<TangleApi::runtime_types::tangle_primitives::services::ServiceBlueprint> {
    let mut blueprint_json = serde_json::to_value(&blueprint)?;
    convert_to_bytes_or_null(&mut blueprint_json["metadata"]["name"]);
    convert_to_bytes_or_null(&mut blueprint_json["metadata"]["description"]);
    convert_to_bytes_or_null(&mut blueprint_json["metadata"]["author"]);
    convert_to_bytes_or_null(&mut blueprint_json["metadata"]["license"]);
    convert_to_bytes_or_null(&mut blueprint_json["metadata"]["website"]);
    convert_to_bytes_or_null(&mut blueprint_json["metadata"]["code_repository"]);
    convert_to_bytes_or_null(&mut blueprint_json["metadata"]["category"]);
    for job in blueprint_json["jobs"].as_array_mut().unwrap() {
        convert_to_bytes_or_null(&mut job["metadata"]["name"]);
        convert_to_bytes_or_null(&mut job["metadata"]["description"]);
    }
    let blueprint = serde_json::from_value(blueprint_json)?;
    Ok(blueprint)
}

fn convert_to_bytes_or_null(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::String(s) => {
            *v = serde_json::Value::Array(s.bytes().map(serde_json::Value::from).collect());
        }
        _ => {}
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
        package.manifest_path.parent().unwrap().join(path).into()
    }
}

/// Finds a package in the workspace to deploy.
fn find_package<'m>(
    metadata: &'m cargo_metadata::Metadata,
    pkg_name: Option<&String>,
) -> Result<&'m cargo_metadata::Package, eyre::Error> {
    match metadata.workspace_members.len() {
        0 => return Err(eyre::eyre!("No packages found in the workspace")),
        1 => metadata
            .packages
            .iter()
            .find(|p| p.id == metadata.workspace_members[0])
            .ok_or_eyre("No package found in the workspace"),
        _more_than_one if pkg_name.is_some() => metadata
            .packages
            .iter()
            .find(|p| pkg_name.is_some_and(|v| &p.name == v))
            .ok_or_eyre("No package found in the workspace with the specified name"),
        _otherwise => {
            eprintln!("Please specify the package to deploy:");
            for package in metadata.packages.iter() {
                eprintln!("Found: {}", package.name);
            }
            eprintln!();
            return Err(eyre::eyre!(
                "The workspace has multiple packages, please specify the package to deploy"
            ));
        }
    }
}
