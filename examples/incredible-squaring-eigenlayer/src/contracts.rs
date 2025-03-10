//! Contract bindings for EigenLayer integration
//!
//! This module provides type-safe bindings for interacting with EigenLayer contracts.

use alloy_sol_types::sol;
use serde::{Deserialize, Serialize};

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    #[derive(Debug, Serialize, Deserialize)]
    SquaringTask,
    "contracts/out/SquaringTask.sol/SquaringTask.json"
);
