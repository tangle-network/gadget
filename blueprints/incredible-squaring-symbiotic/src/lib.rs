use alloy_sol_types::sol;
use gadget_sdk::{job, load_abi};

use alloy_contract::ContractInstance;
use alloy_json_abi::JsonAbi;
use alloy_network::Ethereum;
use alloy_primitives::{FixedBytes, U256};
use serde::{Deserialize, Serialize};
use std::{ops::Deref, sync::OnceLock};

sol!(
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

pub fn noop(_: U256) {
    // This function intentionally does nothing
}

#[derive(Clone)]
pub struct MyContext;

/// Returns x^2 saturating to [`u64::MAX`] if overflow occurs.
#[job(
    id = 0,
    params(x),
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
pub fn xsquare(context: MyContext, x: U256) -> Result<U256, gadget_sdk::Error> {
    Ok(x.saturating_pow(U256::from(2)))
}

/// Converts the event to inputs.
pub fn convert_event_to_inputs(
    event: IncredibleSquaringTaskManager::NewTaskCreated,
    _i: u32,
) -> (U256,) {
    (event.task.numberToBeSquared,)
}
