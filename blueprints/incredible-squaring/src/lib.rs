use async_trait::async_trait;
use gadget_sdk::event_listener::periodic::PeriodicEventListener;
use gadget_sdk::event_listener::EventListener;
use gadget_sdk::job;
use std::convert::Infallible;
//pub mod eigenlayer;

#[derive(Default)]
struct ReturnsZero;

#[async_trait]
impl EventListener<u64> for ReturnsZero {
    async fn next_event(&mut self) -> Option<u64> {
        Some(0)
    }

    async fn handle_event(&mut self, _event: u64) -> std::io::Result<()> {
        Ok(())
    }
}

/// Returns x^2 saturating to [`u64::MAX`] if overflow occurs.
#[job(
    id = 0,
    params(x),
    result(_),
    event_listener(PeriodicEventListener::<6000, ReturnsZero, u64>),
    verifier(evm = "IncredibleSquaringBlueprint")
)]
pub fn xsquare(x: u64) -> Result<u64, Infallible> {
    Ok(x.saturating_pow(2u32))
}
