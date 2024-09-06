use alloy_primitives::{Bytes, U256};
use alloy_sol_types::sol;
use gadget_sdk::job;
// use eigensdk_rs::eigen_utils::crypto::bls::Signature;
// use gadget_common::{module::network::Network, subxt_signer::bip39::serde::Serialize};
// use gadget_sdk::{
//     job,
//     keystore::{backend::GenericKeyStore, ecdsa},
//     network::gossip::GossipHandle,
//     store::LocalDatabase,
// };
// use std::{collections::HashMap, convert::Infallible};
use alloy_contract::{ContractInstance, Interface};
use alloy_network::Ethereum;
use alloy_sol_types::private::alloy_json_abi::JsonAbi;
use std::convert::Infallible;
use std::ops::Deref;
use IncredibleSquaringTaskManager::{
    respondToTaskCall, G1Point, G2Point, NonSignerStakesAndSignature, Task, TaskResponse,
};

/// Returns x^2 saturating to [`u64::MAX`] if overflow occurs.
#[job(
    id = 0,
    params(x),
    result(_),
    verifier(evm = "IncredibleSquaringBlueprint")
)]
pub fn xsquare(x: u64) -> Result<u64, Infallible> {
    Ok(x.saturating_pow(2u32))
}

// Codegen from ABI file to interact with the contract.
sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    IncredibleSquaringTaskManager,
    "contracts/out/IncredibleSquaringTaskManager.sol/IncredibleSquaringTaskManager.json"
);

use crate::IncredibleSquaringTaskManager::IncredibleSquaringTaskManagerInstance;

impl<T, P> Into<ContractInstance<T, P>> for IncredibleSquaringTaskManagerInstance<T, P> {
    fn into(self) -> ContractInstance<T, P> {
        let provider = self.provider();
        ContractInstance::<T, P>::new(
            self.address().clone(),
            *provider,
            Interface::new(JsonAbi::from_json_str(include_str!("../contracts/out/IncredibleSquaringTaskManager.sol/IncredibleSquaringTaskManager.json")).unwrap()),
        )
    }
}

impl<T, P> Deref<Target = ContractInstance<T, P>> for IncredibleSquaringTaskManagerInstance<T, P> {
    type Target = alloy_contract::ContractInstance<T::T, T::P, Ethereum>;

    fn deref(&self) -> &ContractInstance<T, P> {
        let provider = self.provider();
        ContractInstance::<T, P>::new(
            self.address().clone(),
            *provider,
            Interface::new(JsonAbi::from_json_str(include_str!("../contracts/out/IncredibleSquaringTaskManager.sol/IncredibleSquaringTaskManager.json")).unwrap()),
        )
    }
}

/// Returns x^2 saturating to [`u64::MAX`] if overflow occurs.
#[job(
    id = 1,
    params(number_to_be_squared, task_created_block, quorum_numbers, quorum_threshold_percentage),
    result(_),
    event_handler(
        protocol = "eigenlayer",
        instance = IncredibleSquaringTaskManager,
        event = IncredibleSquaringTaskManager::NewTaskCreated,
        event_converter = convert_event_to_inputs,
        callback = IncredibleSquaringTaskManager::IncredibleSquaringTaskManagerCalls::respondToTask
    ),
)]
pub async fn xsquare_eigen(
    // TODO: Add Context
    // ctx: &MyContext,
    number_to_be_squared: U256,
    task_created_block: u32,
    quorum_numbers: Bytes,
    quorum_threshold_percentage: u32,
) -> Result<respondToTaskCall, Infallible> {
    // TODO: Send our task response to Aggregator RPC server
    // TODO: OR we use the gossip protocol to send the response to peers
    // TODO: Where is by BLS key?

    // // 1. Calculate the squared number and save the response
    // let my_msg = MyMessage {
    //     xsquare: number_to_be_squared.saturating_pow(U256::from(2u32)),
    //     signature: Signature::new_zero(),
    // };
    //
    // let responses: HashMap<ecdsa::Public, MyMessage> = HashMap::new();
    // responses.insert(ecdsa::Public::new_zero(), my_msg);
    //
    // // 2. Gossip the squared result BLS signature
    // let handle = &ctx.network;
    // let my_msg_json = serde_json::to_string(&my_msg).unwrap();
    // handle.send_message(my_msg_json).await;
    //
    // // 3. Receive gossiped results from peers
    // let quorum_size = 1;
    // let mut rx = handle.rx_from_inbound.lock().await;
    // while let Some(message) = rx.recv().await {
    //     let message: MyMessage = serde_json::from_str(&message).unwrap();
    //     let public_key = ecdsa::Public::new_zero();
    //     responses.insert(public_key, message);
    //
    //     if responses.len() >= quorum_size {
    //         break;
    //     }
    // }
    //
    // // 4. Once we have a quorum, we calculate the non signers/participants and stakes
    let non_signer_stakes_and_signature: NonSignerStakesAndSignature =
        NonSignerStakesAndSignature {
            nonSignerQuorumBitmapIndices: vec![], // Vec<u32>,
            nonSignerPubkeys: vec![],             // Vec<<G1Point as SolType>::RustType>,
            nonSignerStakeIndices: vec![],        // Vec<u32>,
            quorumApks: vec![],                   // Vec<<G1Point as SolType>::RustType>,
            apkG2: G2Point {
                X: [U256::ZERO; 2], // <G2Point as SolType>::RustType,
                Y: [U256::ZERO; 2], // <G2Point as SolType>::RustType,
            },
            sigma: G1Point {
                X: U256::ZERO, // <G1Point as SolType>::RustType,
                Y: U256::ZERO, // <G1Point as SolType>::RustType,
            },
            quorumApkIndices: vec![],  // Vec<u32>,
            totalStakeIndices: vec![], // Vec<u32>,
        };

    // 5. We submit the full task response
    Ok(respondToTaskCall {
        task: Task {
            numberToBeSquared: number_to_be_squared,
            taskCreatedBlock: task_created_block,
            quorumNumbers: quorum_numbers,
            quorumThresholdPercentage: quorum_threshold_percentage,
        },
        taskResponse: TaskResponse {
            referenceTaskIndex: task_created_block,
            numberSquared: number_to_be_squared.saturating_pow(U256::from(2u32)),
        },
        nonSignerStakesAndSignature: non_signer_stakes_and_signature,
    })
}

/// Converts the event to inputs.
///
/// Uses a tuple to represent the return type because
/// the macro will index all values in the #[job] function
/// and parse the return type by the index.
pub fn convert_event_to_inputs(
    event: IncredibleSquaringTaskManager::NewTaskCreated,
) -> (U256, u32, Bytes, u32) {
    let number_to_be_squared = event.task.numberToBeSquared;
    let task_created_block = event.task.taskCreatedBlock;
    let quorum_numbers = event.task.quorumNumbers;
    let quorum_threshold_percentage = event.task.quorumThresholdPercentage;
    (
        number_to_be_squared,
        task_created_block,
        quorum_numbers,
        quorum_threshold_percentage,
    )
}

#[benchmark(cores = 1, job_id = 0)]
fn xsquare() {
    let _ = xsquare(10).unwrap();
}
