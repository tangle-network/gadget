#![allow(dead_code)]
use crate::contexts::client::{AggregatorClient, SignedTaskResponse};
use crate::{noop, IncredibleSquaringTaskManager, INCREDIBLE_SQUARING_TASK_MANAGER_ABI_STRING};
use alloy_primitives::{hex, Bytes, U256};
use alloy_primitives::{keccak256, FixedBytes};
use alloy_sol_types::SolType;
use ark_bn254::Fq;
use ark_ff::{BigInteger, PrimeField};
use color_eyre::owo_colors::OwoColorize;
use color_eyre::Result;
use eigensdk::crypto_bls::BlsKeyPair;
use eigensdk::crypto_bls::OperatorId;
use eigensdk::types::operator::operator_id_from_g1_pub_key;
use gadget_sdk::ctx::KeystoreContext;
use gadget_sdk::keystore::BackendExt;
use gadget_sdk::{error, info, job};
use std::{convert::Infallible, ops::Deref, sync::OnceLock};
use IncredibleSquaringTaskManager::TaskResponse;

/// Returns x^2 saturating to [`u64::MAX`] if overflow occurs.
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

    // let bls_key_pair = BlsKeyPair::new(
    //     "1371012690269088913462269866874713266643928125698382731338806296762673180359922"
    //         .to_string(),
    // )
    // .unwrap();

    let bls_key_pair = ctx.keystore().unwrap().bls_bn254_key().unwrap();
    let mut operator_id = operator_id_from_g1_pub_key(bls_key_pair.public_key()).unwrap();
    let operator_id = FixedBytes(operator_id);
    println!("Operator ID: {:?}", operator_id);
    // let operator_id: OperatorId =
    //     hex!("fd329fe7e54f459b9c104064efe0172db113a50b5f394949b4ef80b3c34ca7f5").into();

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

/// Helper for converting a PrimeField to its U256 representation for Ethereum compatibility
/// (U256 reads data as big endian)
pub fn point_to_u256(point: Fq) -> U256 {
    let point = point.into_bigint();
    let point_bytes = point.to_bytes_be();
    U256::from_be_slice(&point_bytes[..])
}
