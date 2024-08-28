use gadget_sdk::job;
use std::convert::Infallible;

// TODO: Replace placeholder job with actual Aggregator

#[job(
    id = 0,
    params(x),
    result(_),
    verifier(evm = "IncredibleSquaringBlueprint")
)]
pub fn xsquare(x: u64) -> Result<u64, Infallible> {
    Ok(x.saturating_pow(2u32))
}
