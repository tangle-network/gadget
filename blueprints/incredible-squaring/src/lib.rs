use async_trait::async_trait;
use gadget_sdk::event_listener::EventListener;
use gadget_sdk::{job, Error};
use std::convert::Infallible;
use std::time::Instant;
//pub mod eigenlayer;

pub struct Ticker<const MSEC: usize> {
    additional_delay: u64,
}

#[derive(Copy, Clone)]
pub struct MyContext {
    pub additional_delay: u64,
}

#[async_trait]
impl<const MSEC: usize> EventListener<Instant, MyContext> for Ticker<MSEC> {
    /// Use a reference to the context, [`MyContext`], to construct the listener
    async fn new(context: &MyContext) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Ok(Self {
            additional_delay: context.additional_delay,
        })
    }

    /// Implement the logic that looks for events. In this case, we have a simple stream
    /// that returns after MS milliseconds.
    async fn next_event(&mut self) -> Option<Instant> {
        tokio::time::sleep(tokio::time::Duration::from_millis(
            MSEC as u64 + self.additional_delay,
        ))
        .await;
        Some(Instant::now())
    }

    /// After next_event is called, the event gets passed here. This is where you would implement
    /// listener-specific logic.
    async fn handle_event(&mut self, event: Instant) -> Result<(), Error> {
        gadget_sdk::info!("Event occurred at {event:?}");
        Ok(())
    }
}

/// Returns x^2 saturating to [`u64::MAX`] if overflow occurs.
#[job(
    id = 0,
    params(x),
    result(_),
    event_listener(TangleEventListener, Ticker<6000>),
    verifier(evm = "IncredibleSquaringBlueprint")
)]
pub fn xsquare(x: u64, my_context: MyContext) -> Result<u64, Infallible> {
    Ok(x.saturating_pow(2u32))
}
