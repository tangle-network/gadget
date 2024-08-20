use alloy_sol_types::{sol, SolEvent};
use gadget_sdk::job;
use std::convert::Infallible;

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
    params(x),
    result(_),
    verifier(evm = "IncredibleSquaringTaskManager"),
    event_handler(protocol = "eigenlayer", instance = IncredibleSquaringTaskManager::IncredibleSquaringTaskManagerInstance, event = IncredibleSquaringTaskManager::Event::Verify, callback = IncredibleSquaringTaskManager),
    // event_handler("eigenlayer"), generator = "createTask(uint256)", generate_event = "TaskCreated(uint256)", callback = "submitTask(uint256)")
)]
pub fn xcube(x: u64) -> Result<u64, Infallible> {
    Ok(x.saturating_pow(3u32))
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
