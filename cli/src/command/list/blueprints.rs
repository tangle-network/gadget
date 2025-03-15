use color_eyre::Result;
use dialoguer::console::style;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::service::ServiceBlueprint;
use tangle_subxt::subxt::utils::AccountId32;
use gadget_clients::tangle::client::OnlineClient;
use crate::decode_bounded_string;

/// Lists all blueprints from the Tangle Network.
///
/// # Arguments
///
/// * `ws_rpc_url` - WebSocket RPC URL for the Tangle Network
///
/// # Returns
///
/// A vector of tuples containing blueprint IDs and blueprints.
///
/// # Errors
///
/// Returns an error if:
/// * Failed to connect to the Tangle Network
/// * Failed to query storage
/// * Failed to parse storage data
///
/// # Panics
///
/// * Panics if the key bytes cannot be converted to a blueprint ID
pub async fn list_blueprints(
    ws_rpc_url: String,
) -> Result<Vec<(u64, AccountId32, ServiceBlueprint)>> {
    let client = OnlineClient::from_url(ws_rpc_url.clone()).await?;

    let blueprints_addr = tangle_subxt::tangle_testnet_runtime::api::storage()
        .services()
        .blueprints_iter();

    println!("{}", style("Fetching blueprints...").cyan());
    let mut storage_query = client
        .storage()
        .at_latest()
        .await?
        .iter(blueprints_addr)
        .await?;

    let mut blueprints = Vec::new();

    while let Some(result) = storage_query.next().await {
        let result = result?;
        let blueprint_entry = result.value;
        let owner = blueprint_entry.0;
        let id = u64::from_le_bytes(
            result.key_bytes[32..]
                .try_into()
                .expect("Failed to parse blueprint ID"),
        );
        let blueprint = blueprint_entry.1;
        blueprints.push((id, owner, blueprint));
    }

    println!(
        "{}",
        style(format!("Found {} blueprints", blueprints.len())).green()
    );
    Ok(blueprints)
}

/// Prints the given list of blueprints with their details.
///
/// # Arguments
///
/// * `blueprints` - A vector of tuples containing blueprint IDs and blueprints.
pub fn print_blueprints(blueprints: Vec<(u64, AccountId32, ServiceBlueprint)>) {
    if blueprints.is_empty() {
        println!("{}", style("No blueprints found").yellow());
        return;
    }

    println!("\n{}", style("Blueprints").cyan().bold());
    println!(
        "{}",
        style("=============================================").dim()
    );

    for (blueprint_id, owner, blueprint) in blueprints {
        println!(
            "{}: {}",
            style("Blueprint ID").green().bold(),
            style(blueprint_id).green()
        );
        println!(
            "{}: {}",
            style("Blueprint Owner").green().bold(),
            style(owner).green()
        );

        println!(
            "\n{}",
            style("Blueprint Configuration Information").green().bold()
        );

        println!("\t{}: [", style("Supported Membership Models").green());
        for model in blueprint.supported_membership_models.0.clone() {
            println!("\t\t{}", style(format!("{:?}", model)).yellow());
        }
        println!("\t]");

        println!("\t{}: [", style("Registration Parameters").green());
        for param in blueprint.registration_params.0.clone() {
            println!("\t\t{}", style(format!("{:?}", param)).yellow());
        }
        println!("\t]");

        println!("\t{}: [", style("Request Parameters").green());
        for param in blueprint.request_params.0.clone() {
            println!("\t\t{}", style(format!("{:?}", param)).yellow());
        }
        println!("\t]");

        println!("{}", style("\n======== Jobs ========").green().bold());
        for (job_id, job) in blueprint.jobs.0.iter().enumerate() {
            println!("\n{}:", style(format!("Job ID: {}", job_id)).green().bold());

            let name = decode_bounded_string(&job.metadata.name);
            println!("\t{}: {}", style("Name").green(), style(name).yellow());

            if let Some(desc) = &job.metadata.description {
                let description = decode_bounded_string(desc);
                println!(
                    "\t{}: {}",
                    style("Description").green(),
                    style(description).yellow()
                );
            }

            println!("\t{}: [", style("Parameters").green());
            for param in job.params.0.clone() {
                println!("\t\t{}", style(format!("{:?}", param)).yellow());
            }
            println!("\t]");

            println!("\t{}: [", style("Result").green());
            for result in job.result.0.clone() {
                println!("\t\t{}", style(format!("{:?}", result)).yellow());
            }
            println!("\t]");
        }

        println!(
            "{}",
            style("=============================================").dim()
        );
    }
}
