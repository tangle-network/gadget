use std::convert::Infallible;
use blueprint_sdk::event_listeners::tangle::events::TangleEventListener;
use blueprint_sdk::event_listeners::tangle::services::{services_post_processor, services_pre_processor};
use blueprint_sdk::macros::contexts::{ServicesContext, TangleClientContext};
use blueprint_sdk::macros::ext::tangle::tangle_subxt::tangle_testnet_runtime::api::services::events::JobCalled;
use blueprint_sdk::macros::job;
use blueprint_sdk::config::GadgetConfiguration;
use blueprint_sdk::macros as gadget_macros;

// TODO: Uncomment once tests are working
#[cfg(test)]
mod tests;

#[derive(Clone, TangleClientContext, ServicesContext)]
pub struct MyContext {
    #[config]
    pub env: GadgetConfiguration,
    #[call_id]
    pub call_id: Option<u64>,
}

#[job(
    id = 0,
    params(x),
    event_listener(
        listener = TangleEventListener<MyContext, JobCalled>,
        pre_processor = services_pre_processor,
        post_processor = services_post_processor,
    ),
)]
/// Returns x^2 saturating to [`u64::MAX`] if overflow occurs.
pub fn xsquare(x: u64, _context: MyContext) -> Result<u64, Infallible> {
    Ok(x.saturating_pow(2))
}
