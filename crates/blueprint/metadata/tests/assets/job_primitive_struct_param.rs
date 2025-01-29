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

pub struct SomeParam {
    pub a: u8,
    pub b: u16,
    pub c: u32,
    pub d: u64,
    pub e: u128,
    pub f: i8,
    pub g: i16,
    pub h: i32,
    pub i: i64,
    pub j: i128,
    pub k: f32,
    pub l: f64,
    pub m: bool,
}

#[blueprint_sdk::job(
    id = 0,
    params(x),
    event_listener(
        listener = TangleEventListener<MyContext, JobCalled>,
        pre_processor = services_pre_processor,
        post_processor = services_post_processor,
    ),
)]
pub fn xsquare(x: SomeParam, _context: MyContext) -> Result<u64, Infallible> {
    Ok(x.d.saturating_pow(2))
}
