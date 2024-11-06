use crate::examples::eigen_context::IncredibleSquaringTaskManager::NewTaskCreated;
use alloy_primitives::{Address, Bytes};
use gadget_sdk::event_listener::evm::contracts::EvmContractEventListener;
use gadget_sdk::{config::StdGadgetConfiguration, ctx::EigenlayerContext, job, load_abi};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Deref;

alloy_sol_types::sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    #[derive(Debug, Serialize, Deserialize)]
    IncredibleSquaringTaskManager,
    "contracts/out/IncredibleSquaringTaskManager.sol/IncredibleSquaringTaskManager.json"
);
load_abi!(
    INCREDIBLE_SQUARING_TASK_MANAGER_ABI_STRING,
    "contracts/out/IncredibleSquaringTaskManager.sol/IncredibleSquaringTaskManager.json"
);

#[derive(Clone, EigenlayerContext)]
pub struct ExampleEigenContext {
    #[config]
    pub std_config: StdGadgetConfiguration,
    pub http_rpc_endpoint: String,
    pub ws_rpc_endpoint: String,
}

#[job(
    id = 1,
    event_listener(
        listener = EvmContractEventListener<IncredibleSquaringTaskManager::NewTaskCreated>,
        instance = IncredibleSquaringTaskManager,
        abi = INCREDIBLE_SQUARING_TASK_MANAGER_ABI_STRING,
    ),
)]
pub async fn demonstrate_eigenlayer_context(
    ctx: ExampleEigenContext,
    event: (NewTaskCreated, gadget_sdk::alloy_rpc_types::Log),
) -> Result<u32, Box<dyn std::error::Error>> {
    // Example operator ID and address
    let operator_id = FixedBytes::<32>::from([0u8; 32]);
    let operator_addr = Address::from([0u8; 20]);
    let quorum_number: u8 = 1;
    let block_number: u32 = 100;
    let index: U256 = U256::from(0);

    // Get Operator stake in Quorums at a given block.
    let stake_in_quorums_at_block = ctx
        .get_operator_stake_in_quorums_at_block(block_number, Bytes::from("quorum_numbers"))
        .await?;
    println!("Stake in Quorums at Block: {:?}", stake_in_quorums_at_block);

    // Get an Operator's stake in Quorums at the current block.
    let stake_in_quorums_at_current_block = ctx
        .get_operator_stake_in_quorums_at_current_block(operator_id)
        .await?;
    println!(
        "Stake in Quorums at Current Block: {:?}",
        stake_in_quorums_at_current_block
    );

    // Get an Operator by ID.
    let operator_by_id = ctx.get_operator_by_id(*operator_id).await?;
    println!("Operator by ID: {:?}", operator_by_id);

    // Get an Operator stake history.
    let stake_history = ctx
        .get_operator_stake_history(operator_id, quorum_number)
        .await?;
    println!("Stake History for {operator_id} in Quorum {quorum_number}:");
    for (update_num, stake_update) in stake_history.as_slice().iter().enumerate() {
        println!("\tStake Update {update_num}: \n\t\tStake: {:?},\n\t\tUpdate Block Number: {:?},\n\t\tNext Update Block Number: {:?}",
                 stake_update.stake,
                 stake_update.updateBlockNumber,
                 stake_update.nextUpdateBlockNumber);
    }

    // Get an Operator stake update at a given index.
    let stake_update_at_index = ctx
        .get_operator_stake_update_at_index(quorum_number, operator_id, index)
        .await?;
    println!("Stake Update at Index {index}: \n\tStake: {:?}\n\tUpdate Block Number: {:?}\n\tNext Update Block Number: {:?}", stake_update_at_index.stake, stake_update_at_index.updateBlockNumber, stake_update_at_index.nextUpdateBlockNumber);

    // Get an Operator's stake at a given block number.
    let stake_at_block_number = ctx
        .get_operator_stake_at_block_number(operator_id, quorum_number, block_number)
        .await?;
    println!("Stake at Block Number: {:?}", stake_at_block_number);

    // Get an Operator's details.
    let operator = ctx.get_operator_details(operator_addr).await?;
    println!("Operator Details: \n\tAddress: {:?},\n\tEarnings receiver address: {:?},\n\tDelegation approver address: {:?},\n\tMetadata URL: {:?},\n\tStaker Opt Out Window Blocks: {:?}",
             operator.address,
             operator.earnings_receiver_address,
             operator.delegation_approver_address,
             operator.metadata_url,
             operator.staker_opt_out_window_blocks);

    // Get an Operator's latest stake update.
    let latest_stake_update = ctx
        .get_latest_stake_update(operator_id, quorum_number)
        .await?;
    println!("Latest Stake Update: \n\tStake: {:?},\n\tUpdate Block Number: {:?},\n\tNext Update Block Number: {:?}",
             latest_stake_update.stake,
             latest_stake_update.updateBlockNumber,
             latest_stake_update.nextUpdateBlockNumber);

    // Get an Operator's ID as FixedBytes from its Address.
    let operator_id_from_address = ctx.get_operator_id(operator_addr).await?;
    println!("Operator ID from Address: {:?}", operator_id_from_address);

    // Get the total stake at a given block number from a given index.
    let total_stake_at_block_number_from_index = ctx
        .get_total_stake_at_block_number_from_index(quorum_number, block_number, index)
        .await?;
    println!(
        "Total Stake at Block Number from Index: {:?}",
        total_stake_at_block_number_from_index
    );

    // Get the total stake history length of a given quorum.
    let total_stake_history_length = ctx.get_total_stake_history_length(quorum_number).await?;
    println!(
        "Total Stake History Length: {:?}",
        total_stake_history_length
    );

    // Provides the public keys of existing registered operators within the provided block range.
    let existing_registered_operator_pub_keys = ctx
        .query_existing_registered_operator_pub_keys(0, block_number as u64)
        .await?;
    println!(
        "Existing Registered Operator Public Keys: {:?}",
        existing_registered_operator_pub_keys
    );

    Ok(0)
}

// pub async fn handle_inputs(
//     event: (
//         IncredibleSquaringTaskManager::NewTaskCreated,
//         alloy_rpc_types::Log,
//     ),
// ) -> Result<u32, gadget_sdk::Error> {
//     let task_index = event.0.taskIndex;
//     println!("Task Index: {task_index}");
//     Ok(task_index)
// }

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     // Example configuration
//     let config = StdGadgetConfiguration::default();
//     let context = ExampleEigenContext {
//         std_config: config,
//         http_rpc_endpoint: "http://localhost:8545".to_string(),
//         ws_rpc_endpoint: "ws://localhost:8546".to_string(),
//     };
//
//     demonstrate_eigenlayer_context(&context).await
// }
