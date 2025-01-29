use std::convert::Infallible;
use blueprint_sdk::event_listeners::tangle::events::TangleEventListener;
use blueprint_sdk::event_listeners::tangle::services::{services_post_processor, services_pre_processor};
use blueprint_sdk::macros::contexts::{ServicesContext, TangleClientContext};
use blueprint_sdk::macros::ext::tangle::tangle_subxt::tangle_testnet_runtime::api::services::events::JobCalled;
use blueprint_sdk::config::GadgetConfiguration;

#[derive(Clone, TangleClientContext, ServicesContext)]
pub struct MyContext {
    #[config]
    pub env: GadgetConfiguration,
    #[call_id]
    pub call_id: Option<u64>,
}

#[derive(PartialEq)]
pub enum Status {
    Failed,
    Success,
}

#[blueprint_sdk::job(
    id = 0,
    params(status),
    event_listener(
        listener = TangleEventListener<MyContext, JobCalled>,
        pre_processor = services_pre_processor,
        post_processor = services_post_processor,
    ),
)]
pub fn check_status(status: Status, _context: MyContext) -> Result<bool, Infallible> {
    Ok(status == Status::Success)
}
