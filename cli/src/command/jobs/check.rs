use color_eyre::Result;
use dialoguer::console::style;
use gadget_clients::tangle::client::OnlineClient;
use std::time::Duration;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::FieldType;
use tangle_subxt::tangle_testnet_runtime::api::services::events::JobResultSubmitted;

use crate::command::jobs::helpers::print_job_results;

/// Waits for the completion of a Tangle job and prints the results.
///
/// # Arguments
///
/// * `ws_rpc_url` - WebSocket RPC URL for the Tangle Network
/// * `service_id` - ID of the service the job was submitted to
/// * `call_id` - ID of the job call to wait for
/// * `timeout_seconds` - Maximum time to wait for job completion in seconds (0 for no timeout)
///
/// # Errors
///
/// Returns an error if:
/// * Failed to connect to the Tangle Network
/// * Failed to subscribe to block events
/// * Timeout reached before job completion
/// * No job result found
pub async fn check_job(
    ws_rpc_url: String,
    service_id: u64,
    call_id: u64,
    result_types: Vec<FieldType>,
    timeout_seconds: u64,
) -> Result<()> {
    let client = OnlineClient::from_url(ws_rpc_url.clone()).await?;

    println!(
        "{}",
        style(format!(
            "Waiting for completion of job {} on service {}...",
            call_id, service_id
        ))
        .cyan()
    );

    // Set up timeout if specified
    let timeout = if timeout_seconds > 0 {
        Some(Duration::from_secs(timeout_seconds))
    } else {
        None
    };

    // Subscribe to new blocks
    let mut count = 0;
    let mut blocks = client.blocks().subscribe_best().await?;

    let start_time = std::time::Instant::now();

    while let Some(Ok(block)) = blocks.next().await {
        // Check for timeout
        if let Some(timeout_duration) = timeout {
            if start_time.elapsed() > timeout_duration {
                println!(
                    "{}",
                    style("Timeout reached while waiting for job results")
                        .yellow()
                        .bold()
                );
                return Err(color_eyre::eyre::eyre!(
                    "Timeout reached while waiting for job completion"
                ));
            }
        }

        // Print progress periodically
        if count % 10 == 0 {
            println!(
                "{}",
                style(format!(
                    "Checking block #{} for job results (elapsed: {:?})...",
                    block.number(),
                    start_time.elapsed()
                ))
                .dim()
            );
        }
        count += 1;

        // Check for job result events in this block
        let events = block.events().await?;
        let results = events.find::<JobResultSubmitted>().collect::<Vec<_>>();

        for result in results {
            match result {
                Ok(result) => {
                    if result.service_id == service_id && result.call_id == call_id {
                        println!(
                            "\n{}",
                            style(format!(
                                "Job completed successfully on service {} with call ID {}",
                                service_id, call_id
                            ))
                            .green()
                            .bold()
                        );

                        println!("\n{}", style("Job Results:").cyan().bold());
                        println!(
                            "{}",
                            style("=============================================").dim()
                        );

                        if result.result.is_empty() {
                            println!("{}", style("No output values returned").yellow());
                        } else {
                            for (i, output) in result.result.iter().enumerate() {
                                print_job_results(&result_types, i, output.clone());
                            }
                        }

                        println!(
                            "{}",
                            style("=============================================").dim()
                        );
                        return Ok(());
                    }
                }
                Err(err) => {
                    println!(
                        "{}",
                        style(format!("Error processing event: {}", err)).red()
                    );
                }
            }
        }
    }

    println!(
        "{}",
        style("Block subscription ended without finding job results")
            .yellow()
            .bold()
    );
    Err(color_eyre::eyre::eyre!("Failed to find job results"))
}
