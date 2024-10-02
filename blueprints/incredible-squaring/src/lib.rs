use async_trait::async_trait;
use gadget_sdk::config::GadgetConfiguration;
use gadget_sdk::event_listener::EventListener;
use gadget_sdk::{job, Error};
use std::convert::Infallible;
use std::time::Instant;
//pub mod eigenlayer;

/// Define a simple event listener that ticks every MS milliseconds.
pub struct Ticker<const MS: usize> {
    additional_delay: u64,
}

#[derive(Copy, Clone)]
pub struct MyContext {
    pub additional_delay: u64,
}

/// Implement the [`EventListener`] trait for the Ticker struct. [`EventListener`] has two type parameters:
///
/// - T: is the type of the event that the listener listens for. This can be anything that is Send + Sync + 'static.
/// In this case, we are using [`Instant`], which is a timestamp.
///
/// - Ctx: is the context type that the listener receives when constructed. This can be anything that is Send + Sync + 'static,
/// with the special requirement that it is the *first* listed additional parameter in the [`job`] macro (i.e., a parameter not
/// in params(...)). In this case, we are using [`MyContext`].
#[async_trait]
impl<const MS: usize> EventListener<Instant, MyContext> for Ticker<MS> {
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
            MS as u64 + self.additional_delay,
        ))
        .await;
        Some(Instant::now())
    }

    /// After next_event is called, the event gets passed here. This is where you would implement
    /// listener-specific logic.
    async fn handle_event(&mut self, _event: Instant) -> Result<(), Error> {
        Ok(())
    }
}

/// Returns x^2 saturating to [`u64::MAX`] if overflow occurs.
#[job(
    id = 0,
    params(x),
    result(_),
    event_listener(Ticker::<6000>),
    verifier(evm = "IncredibleSquaringBlueprint")
)]
pub fn xsquare(
    context: MyContext,
    x: u64,
    env: GadgetConfiguration<parking_lot::RawRwLock>,
) -> Result<u64, Infallible> {
    Ok(x.saturating_pow(2u32))
}
