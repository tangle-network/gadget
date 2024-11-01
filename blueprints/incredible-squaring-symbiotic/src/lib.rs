use alloy_primitives::U256;
use alloy_sol_types::sol;
use gadget_sdk::event_listener::evm::contracts::EvmContractEventListener;
use gadget_sdk::{job, load_abi};
use serde::{Deserialize, Serialize};
use std::ops::Deref;

sol!(
    #[allow(missing_docs, clippy::too_many_arguments)]
    #[sol(rpc)]
    #[derive(Debug, Serialize, Deserialize)]
    IncredibleSquaringTaskManager,
    "contracts/out/IncredibleSquaringTaskManager.sol/IncredibleSquaringTaskManager.json"
);

load_abi!(
    INCREDIBLE_SQUARING_TASK_MANAGER_ABI_STRING,
    "contracts/out/IncredibleSquaringTaskManager.sol/IncredibleSquaringTaskManager.json"
);

pub async fn noop(_: U256) -> Result<(), gadget_sdk::Error> {
    // This function intentionally does nothing
    Ok(())
}

#[derive(Clone)]
pub struct MyContext;

/// Returns x^2 saturating to [`u64::MAX`] if overflow occurs.
#[job(
    id = 0,
    params(x),
    event_listener(
        listener = EvmContractEventListener<IncredibleSquaringTaskManager::NewTaskCreated>,
        instance = IncredibleSquaringTaskManager,
        pre_processor = convert_event_to_inputs,
        abi = INCREDIBLE_SQUARING_TASK_MANAGER_ABI_STRING,
        post_processor = noop,
    ),
)]
pub fn xsquare(x: U256, context: MyContext) -> Result<U256, gadget_sdk::Error> {
    Ok(x.saturating_pow(U256::from(2)))
}

/// Converts the event to inputs.
pub async fn convert_event_to_inputs(
    (event, _log): (
        IncredibleSquaringTaskManager::NewTaskCreated,
        alloy_rpc_types::Log,
    ),
) -> Result<(U256,), gadget_sdk::Error> {
    Ok((event.task.numberToBeSquared,))
}
