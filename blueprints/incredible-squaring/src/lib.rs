use async_trait::async_trait;
use gadget_sdk::clients::tangle::runtime::TangleConfig;
use gadget_sdk::config::GadgetConfiguration;
use gadget_sdk::event_listener::TangleEventListener;
use gadget_sdk::ext::subxt::blocks::Block;
use gadget_sdk::ext::subxt::OnlineClient;
use gadget_sdk::{job, Error};
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
