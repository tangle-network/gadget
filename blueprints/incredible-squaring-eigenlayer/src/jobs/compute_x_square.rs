#![allow(dead_code)]
use crate::contexts::client::{AggregatorClient, SignedTaskResponse};
use crate::{noop, IncredibleSquaringTaskManager, INCREDIBLE_SQUARING_TASK_MANAGER_ABI_STRING};
use alloy_primitives::keccak256;
use alloy_primitives::{Bytes, U256};
use alloy_sol_types::SolType;
use color_eyre::Result;
use gadget_sdk::ctx::KeystoreContext;
use gadget_sdk::keystore::BackendExt;
use gadget_sdk::runners::eigenlayer::derive_operator_id;
use gadget_sdk::{error, info, job};
use std::{convert::Infallible, ops::Deref, sync::OnceLock};
use IncredibleSquaringTaskManager::TaskResponse;

/// Sends a signed task response to the BLS Aggregator.
/// This job is triggered by the `NewTaskCreated` event emitted by the `IncredibleSquaringTaskManager`.
/// The job calculates the square of the number to be squared and sends the signed task response to the BLS Aggregator.
/// The job returns 1 if the task response was sent successfully.
/// The job returns 0 if the task response failed to send or failed to get the BLS key.
#[job(
    id = 0,
    params(number_to_be_squared, task_created_block, quorum_numbers, quorum_threshold_percentage, task_index),
    result(_),
    event_listener(
        listener = EvmContractEventListener(
            instance = IncredibleSquaringTaskManager,
            abi = INCREDIBLE_SQUARING_TASK_MANAGER_ABI_STRING,
        ),
        event = IncredibleSquaringTaskManager::NewTaskCreated,
        pre_processor = convert_event_to_inputs,
        post_processor = noop,
    ),
)]
pub async fn xsquare_eigen(
    ctx: AggregatorClient,
    number_to_be_squared: U256,
    task_created_block: u32,
    quorum_numbers: Bytes,
    quorum_threshold_percentage: u8,
    task_index: u32,
) -> Result<u32, Infallible> {
    // Calculate our response to job
    let task_response = TaskResponse {
        referenceTaskIndex: task_index,
        numberSquared: number_to_be_squared.saturating_pow(U256::from(2u32)),
    };

    let bls_key_pair = match ctx.keystore().and_then(|ks| Ok(ks.bls_bn254_key())) {
        Ok(kp) => match kp {
            Ok(k) => k,
            Err(e) => return Ok(0),
        },
        Err(e) => return Ok(0),
    };
    let operator_id = derive_operator_id(bls_key_pair.public_key());

    // Sign the Hashed Message and send it to the BLS Aggregator
    let msg_hash = keccak256(<TaskResponse as SolType>::abi_encode(&task_response));
    let signed_response = SignedTaskResponse {
        task_response,
        signature: bls_key_pair.sign_message(msg_hash.as_ref()),
        operator_id,
    };

    info!(
        "Sending signed task response to BLS Aggregator: {:#?}",
        signed_response
    );
    if let Err(e) = ctx.send_signed_task_response(signed_response).await {
        error!("Failed to send signed task response: {:?}", e);
        return Ok(0);
    }

    Ok(1)
}

/// Converts the event to inputs.
///
/// Uses a tuple to represent the return type because
/// the macro will index all values in the #[job] function
/// and parse the return type by the index.
pub fn convert_event_to_inputs(
    event: IncredibleSquaringTaskManager::NewTaskCreated,
    _index: u32,
) -> (U256, u32, Bytes, u8, u32) {
    let task_index = event.taskIndex;
    let number_to_be_squared = event.task.numberToBeSquared;
    let task_created_block = event.task.taskCreatedBlock;
    let quorum_numbers = event.task.quorumNumbers;
    let quorum_threshold_percentage = event.task.quorumThresholdPercentage.try_into().unwrap();
    (
        number_to_be_squared,
        task_created_block,
        quorum_numbers,
        quorum_threshold_percentage,
        task_index,
    )
}
