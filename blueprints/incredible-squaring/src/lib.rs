use gadget_sdk::event_listener::tangle::jobs::{services_post_processor, services_pre_processor};
use gadget_sdk::event_listener::tangle::TangleEventListener;
use gadget_sdk::job;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::services::events::JobCalled;

#[derive(Clone)]
pub struct MyContext;

/// Returns x^2 saturating to [`u64::MAX`] if overflow occurs.
#[job(
    id = 0,
    params(x),
    result(_),
    event_listener(
        listener = TangleEventListener<JobCalled, MyContext>,
        pre_processor = services_pre_processor,
        post_processor = services_post_processor,
    ),
)]
pub fn xsquare(x: u64, context: MyContext) -> Result<u64, gadget_sdk::Error> {
    Ok(x.saturating_pow(2))
}
