use gadget_sdk::contexts::TangleClientContext;
use gadget_sdk::event_listener::tangle::jobs::{services_post_processor, services_pre_processor};
use gadget_sdk::event_listener::tangle::TangleEventListener;
use gadget_sdk::job;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::services::events::JobCalled;

#[derive(Clone, TangleClientContext)]
pub struct MyContext {
    #[config]
    pub config: gadget_sdk::config::StdGadgetConfiguration,
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
pub fn xsquare(x: u64, context: MyContext) -> Result<u64, gadget_sdk::Error> {
    assert!(context.call_id.is_some());
    Ok(x.saturating_pow(2))
}
