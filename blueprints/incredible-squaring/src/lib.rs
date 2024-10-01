use async_trait::async_trait;
use gadget_sdk::config::GadgetConfiguration;
use gadget_sdk::event_listener::periodic::PeriodicEventListener;
use gadget_sdk::event_listener::EventListener;
use gadget_sdk::{job, Error};
use std::convert::Infallible;
//pub mod eigenlayer;

struct ReturnsZero;

#[async_trait]
impl EventListener<u64, MyContext> for ReturnsZero {
    async fn new(_context: &MyContext) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Ok(Self)
    }

    async fn next_event(&mut self) -> Option<u64> {
        Some(0)
    }

    async fn handle_event(&mut self, _event: u64) -> std::io::Result<()> {
        Ok(())
    }
}

#[derive(Copy, Clone)]
pub struct MyContext;

/// Returns x^2 saturating to [`u64::MAX`] if overflow occurs.
#[job(
    id = 0,
    params(x),
    result(_),
    event_listener(PeriodicEventListener::<6000, ReturnsZero, u64, MyContext>),
    verifier(evm = "IncredibleSquaringBlueprint")
)]
pub fn xsquare(
    context: MyContext,
    x: u64,
    env: GadgetConfiguration<parking_lot::RawRwLock>,
) -> Result<u64, Infallible> {
    Ok(x.saturating_pow(2u32))
}
