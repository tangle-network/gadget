use alloy_sol_types::sol;
use blueprint_sdk::alloy::primitives::U256;
use blueprint_sdk::event_listeners::evm::EvmContractEventListener;
use blueprint_sdk::macros::load_abi;
use blueprint_sdk::{job, Error};
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

#[derive(Clone)]
pub struct MyContext;

/// Returns x^2 saturating to [`u64::MAX`] if overflow occurs.
#[job(
    id = 0,
    params(x),
    event_listener(
        listener = EvmContractEventListener<IncredibleSquaringTaskManager::NewTaskCreated>,
        instance = IncredibleSquaringTaskManager,
        abi = INCREDIBLE_SQUARING_TASK_MANAGER_ABI_STRING,
        pre_processor = convert_event_to_inputs,
    ),
)]
pub fn xsquare(x: U256, context: MyContext) -> Result<U256, Error> {
    Ok(x.saturating_pow(U256::from(2)))
}

/// Converts the event to inputs.
pub async fn convert_event_to_inputs(
    (event, _log): (
        IncredibleSquaringTaskManager::NewTaskCreated,
        alloy_rpc_types::Log,
    ),
) -> Result<(U256,), Error> {
    Ok((event.task.numberToBeSquared,))
}
