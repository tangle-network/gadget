use alloy_sol_types::sol;
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
    event_handler(
        protocol = "eigenlayer",
        instance = IncredibleSquaringTaskManager,
        event = IncredibleSquaringTaskManager::NewTaskCreated,
        callback = IncredibleSquaringTaskManager::IncredibleSquaringTaskManagerCalls::respondToTask
    ),
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
