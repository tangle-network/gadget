use gadget_blueprint_serde::job;
use gadget_client_tangle::api::services::events::JobCalled;
use gadget_contexts::tangle::TangleClientContext;
use gadget_event_listeners_tangle::{
    events::TangleEventListener,
    services::{services_post_processor, services_pre_processor},
};
use gadget_std::error::Error;

#[cfg(test)]
mod tests;

#[derive(Clone)]
pub struct MyContext;

impl TangleClientContext for MyContext {}

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
pub fn xsquare(x: u64, _context: MyContext) -> Result<u64, Error> {
    Ok(x.saturating_pow(2))
}
