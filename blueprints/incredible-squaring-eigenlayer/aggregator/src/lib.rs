use alloy_sol_types::sol;
use serde::{Deserialize, Serialize};

pub mod aggregator;

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    #[derive(Debug, Serialize, Deserialize)]
    IncredibleSquaringTaskManager,
    "../contracts/out/IncredibleSquaringTaskManager.sol/IncredibleSquaringTaskManager.json"
);
