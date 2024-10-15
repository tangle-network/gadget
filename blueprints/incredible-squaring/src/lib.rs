use gadget_sdk::job;
use std::convert::Infallible;
use gadget_sdk::event_listener::TangleEventListener;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::services::events::JobCalled;


/// Returns x^2 saturating to [`u64::MAX`] if overflow occurs.
#[job(
    id = 0,
    params(x),
    result(_),
    event_listener(TangleEventListener::<JobCalled>),
    verifier(evm = "IncredibleSquaringBlueprint")
)]
pub fn xsquare(x: u64) -> Result<u64, Infallible> {
    Ok(x.saturating_pow(2u32))
}
