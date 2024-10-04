use gadget_sdk::config::GadgetConfiguration;
use gadget_sdk::job;
use std::convert::Infallible;
//pub mod eigenlayer;
/// Returns x^2 saturating to [`u64::MAX`] if overflow occurs.
#[job(
    id = 0,
    params(x),
    result(_),
    event_listener(TangleEventListener),
    verifier(evm = "IncredibleSquaringBlueprint")
)]
pub fn xsquare(
    x: u64,
    env: GadgetConfiguration<parking_lot::RawRwLock>,
) -> Result<u64, Infallible> {
    Ok(x.saturating_pow(2u32))
}
