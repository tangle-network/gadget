use color_eyre::eyre::{self, OptionExt, Result};
use gadget_blueprint_proc_macro_core::{
    JobResultVerifier, ServiceBlueprint, ServiceRegistrationHook, ServiceRequestHook,
};
use tangle_subxt::subxt::{self, PolkadotConfig};
use tangle_subxt::tangle_testnet_runtime::api as TangleApi;

#[derive(Debug, Clone)]
pub struct Opts {
    /// The name of the package to deploy (if the workspace has multiple packages)
    pub pkg_name: Option<String>,
    /// The RPC URL of the Tangle Network
    pub rpc_url: String,
    /// The path to the manifest file
    pub manifest_path: std::path::PathBuf,
}

pub async fn deploy_to_tangle(
    Opts {
        pkg_name,
        rpc_url,
        manifest_path,
    }: Opts,
) -> Result<()> {
    // Load the manifest file into cargo metadata
    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(manifest_path)
        .no_deps()
        .exec()?;
    let package = find_package(&metadata, pkg_name.as_ref())?;
    let blueprint_json_path = package
        .manifest_path
        .parent()
        .map(|p| p.join("blueprint.json"))
        .ok_or_eyre("Could not find blueprint.json; did you run `cargo build`?")?;

    let blueprint_json = std::fs::read_to_string(blueprint_json_path)?;
    let mut blueprint = serde_json::from_str::<ServiceBlueprint<'_>>(&blueprint_json)?;

    // TODO: Deploy Contracts.
    for job in blueprint.jobs.iter_mut() {
        job.verifier = JobResultVerifier::None;
    }
    blueprint.request_hook = ServiceRequestHook::None;
    blueprint.registration_hook = ServiceRegistrationHook::None;

    let blueprint = bake_blueprint(blueprint)?;

    let client = subxt::OnlineClient::<PolkadotConfig>::from_url(rpc_url).await?;
    let blueprint_id = client
        .storage()
        .at_latest()
        .await?
        .fetch_or_default(&TangleApi::storage().services().next_blueprint_id())
        .await?;

    let create_blueprint_tx = TangleApi::tx().services().create_blueprint(blueprint);

    eprintln!("Creating Blueprint: {:?}", create_blueprint_tx);
    // TODO: get the signer from the user.
    let signer = tangle_subxt::subxt_signer::sr25519::dev::alice();
    let progress = client
        .tx()
        .sign_and_submit_then_watch_default(&create_blueprint_tx, &signer)
        .await?;
    let result = progress.wait_for_finalized_success().await?;
    eprintln!(
        "Blueprint #{blueprint_id} Created: {:?}",
        result.extrinsic_hash()
    );
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
