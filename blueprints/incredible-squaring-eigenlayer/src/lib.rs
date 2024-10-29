#![allow(dead_code)]
use alloy_sol_types::sol;
use gadget_sdk::load_abi;
use serde::{Deserialize, Serialize};

pub mod constants;
pub mod contexts;
pub mod jobs;
// pub mod runner;

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

pub fn noop(_: u32) {
    // This function intentionally does nothing
}
