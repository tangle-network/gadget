use color_eyre::Result;
use dialoguer::console::style;
use gadget_clients::tangle::client::OnlineClient;
use tangle_subxt::subxt::utils::AccountId32;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::service::ServiceRequest;

/// Lists all service requests from the Tangle Network.
///
/// # Arguments
///
/// * `ws_rpc_url` - WebSocket RPC URL for the Tangle Network
///
/// # Returns
///
/// A vector of tuples containing request IDs and service requests.
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
/// Panics if the key bytes cannot be converted to a request ID.
pub async fn list_requests(
    ws_rpc_url: String,
) -> Result<Vec<(u64, ServiceRequest<AccountId32, u64, u128>)>> {
    let client = OnlineClient::from_url(ws_rpc_url.clone()).await?;

    let service_requests_addr = tangle_subxt::tangle_testnet_runtime::api::storage()
        .services()
        .service_requests_iter();

    println!("{}", style("Fetching service requests...").cyan());
    let mut storage_query = client
        .storage()
        .at_latest()
        .await?
        .iter(service_requests_addr)
        .await?;

    let mut requests = Vec::new();

    while let Some(result) = storage_query.next().await {
        let result = result?;
        let request = result.value;
        let id = u64::from_le_bytes(
            result.key_bytes[32..]
                .try_into()
                .expect("Invalid key bytes format"),
        );
        requests.push((id, request));
    }

    println!(
        "{}",
        style(format!("Found {} service requests", requests.len())).green()
    );
    Ok(requests)
}

/// Prints the given list of service requests with their details.
///
/// # Arguments
///
/// * `requests` - A vector of tuples containing request IDs and service requests.
pub fn print_requests(requests: Vec<(u64, ServiceRequest<AccountId32, u64, u128>)>) {
    if requests.is_empty() {
        println!("{}", style("No service requests found").yellow());
        return;
    }

    println!("\n{}", style("Service Requests").cyan().bold());
    println!(
        "{}",
        style("=============================================").dim()
    );

    for (request_id, request) in requests {
        println!(
            "{}: {}",
            style("Request ID").green().bold(),
            style(request_id).green()
        );
        println!("{}: {}", style("Blueprint ID").green(), request.blueprint);
        println!("{}: {}", style("Owner").green(), request.owner);
        println!(
            "{}: {:?}",
            style("Permitted Callers").green(),
            request.permitted_callers
        );
        println!(
            "{}: {:?}",
            style("Security Requirements").green(),
            request.security_requirements
        );
        println!(
            "{}: {:?}",
            style("Membership Model").green(),
            request.membership_model
        );
        println!("{}: {:?}", style("Request Arguments").green(), request.args);
        println!("{}: {:?}", style("TTL").green(), request.ttl);
        println!(
            "{}",
            style("=============================================").dim()
        );
    }
}
