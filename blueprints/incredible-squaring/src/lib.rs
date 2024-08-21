use alloy_primitives::U256;
use alloy_sol_types::sol;
use gadget_sdk::job;
use std::convert::Infallible;
use IncredibleSquaringTaskManager::{
    respondToTaskCall, NonSignerStakesAndSignature, Task, TaskResponse,
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

/// Returns x^3 saturating to [`u64::MAX`] if overflow occurs.
#[job(
    id = 1,
    params(task, non_signer_stakes_and_signature),
    result(_),
    event_handler(
        protocol = "eigenlayer",
        instance = IncredibleSquaringTaskManager,
        event = IncredibleSquaringTaskManager::NewTaskCreated,
        callback = IncredibleSquaringTaskManager::IncredibleSquaringTaskManagerCalls::respondToTask
    ),
)]
pub fn xsquare_eigen(
    task: Task,
    non_signer_stakes_and_signature: NonSignerStakesAndSignature,
) -> Result<respondToTaskCall, Infallible> {
    Ok(respondToTaskCall {
        task: task.clone(),
        taskResponse: TaskResponse {
            referenceTaskIndex: task.taskCreatedBlock,
            numberSquared: task.numberToBeSquared.saturating_pow(U256::from(2u32)),
        },
        nonSignerStakesAndSignature: non_signer_stakes_and_signature,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let x = 3;
        assert_eq!(xsquare(x).unwrap(), 9);

        let x = u64::MAX;
        assert_eq!(xsquare(x).unwrap(), u64::MAX);
    }
}
