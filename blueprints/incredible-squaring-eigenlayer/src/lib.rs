#![allow(dead_code)]
use alloy_sol_types::sol;
use gadget_sdk::load_abi;
use serde::{Deserialize, Serialize};

pub mod constants;
pub mod contexts;
pub mod jobs;
#[cfg(test)]
mod tests;

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

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    #[derive(Debug)]
    PauserRegistry,
    "./contracts/out/IPauserRegistry.sol/IPauserRegistry.json"
);

sol!(
    #[allow(missing_docs, clippy::too_many_arguments)]
    #[sol(rpc)]
    #[derive(Debug)]
    RegistryCoordinator,
    "./contracts/out/RegistryCoordinator.sol/RegistryCoordinator.json"
);
