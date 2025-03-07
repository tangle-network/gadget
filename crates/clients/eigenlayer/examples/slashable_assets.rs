use alloy_primitives::{address, Address};
use gadget_client_eigenlayer::client::EigenlayerClient;
use gadget_config::{GadgetConfiguration, Protocol, ProtocolSettings};
use tokio::main;

#[main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a simple configuration with default settings
    let config = GadgetConfiguration::default();

    // Create the EigenlayerClient
    let client = EigenlayerClient::new(config);

    // Example AVS address (replace with your actual AVS address)
    let avs_address = address!("0000000000000000000000000000000000000000");

    // Example block number
    let block_number = 100;

    // Example quorum numbers
    let quorum_numbers = vec![0, 1];

    println!("Getting slashable assets for AVS: {}", avs_address);

    // Get all slashable assets for the AVS
    match client
        .get_slashable_assets_for_avs(avs_address, block_number, quorum_numbers)
        .await
    {
        Ok(assets) => {
            println!("Slashable assets found for {} operators", assets.len());

            // Print detailed information about each operator's slashable assets
            for (operator, strategies) in assets {
                println!("Operator: {}", operator);

                let mut total_slashable = 0u128;

                for (strategy, amount) in strategies {
                    println!("  Strategy: {}", strategy);
                    println!("  Amount slashable: {}", amount);

                    // Convert U256 to u128 for summing (this may lose precision for very large values)
                    if let Ok(amount_u128) = amount.to_string().parse::<u128>() {
                        total_slashable += amount_u128;
                    }
                }

                println!("  Total slashable for operator: {}", total_slashable);
                println!();
            }
        }
        Err(e) => eprintln!("Error getting slashable assets: {}", e),
    }

    // Example: Get strategies in a specific operator set
    let operator_set_id = 0u8;
    match client
        .get_strategies_in_operator_set(avs_address, operator_set_id)
        .await
    {
        Ok(strategies) => {
            println!(
                "Strategies in operator set {} for AVS {}:",
                operator_set_id, avs_address
            );
            for (i, strategy) in strategies.iter().enumerate() {
                println!("  {}: {}", i + 1, strategy);
            }
        }
        Err(e) => eprintln!("Error getting strategies: {}", e),
    }

    // Example: Get strategy allocations for an operator
    // Replace with actual operator and strategy addresses
    let operator_address = address!("0000000000000000000000000000000000000001");
    let strategy_address = address!("0000000000000000000000000000000000000002");

    match client
        .get_strategy_allocations(operator_address, strategy_address)
        .await
    {
        Ok((operator_sets, allocations)) => {
            println!(
                "Allocations for operator {} and strategy {}:",
                operator_address, strategy_address
            );

            for (i, (op_set, allocation)) in
                operator_sets.iter().zip(allocations.iter()).enumerate()
            {
                println!("  Allocation {}:", i + 1);
                println!("    AVS: {}", op_set.avs);
                println!("    Operator Set ID: {}", op_set.id);
                println!("    Current Magnitude: {}", allocation.currentMagnitude);
                println!("    Pending Diff: {}", allocation.pendingDiff);
                println!("    Effect Block: {}", allocation.effectBlock);
            }
        }
        Err(e) => eprintln!("Error getting strategy allocations: {}", e),
    }

    // Example: Get maximum magnitude for an operator and strategy
    match client
        .get_max_magnitude(operator_address, strategy_address)
        .await
    {
        Ok(max_magnitude) => {
            println!(
                "Maximum magnitude for operator {} and strategy {}: {}",
                operator_address, strategy_address, max_magnitude
            );
        }
        Err(e) => eprintln!("Error getting max magnitude: {}", e),
    }

    // Example: Get slashable shares directly using DelegationManager
    match client
        .get_slashable_shares_in_queue(operator_address, strategy_address)
        .await
    {
        Ok(slashable_shares) => {
            println!(
                "Slashable shares for operator {} and strategy {}: {}",
                operator_address, strategy_address, slashable_shares
            );
        }
        Err(e) => eprintln!("Error getting slashable shares: {}", e),
    }

    Ok(())
}
