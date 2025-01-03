pub mod error;
use error::Error;

use async_trait::async_trait;
use gadget_event_listeners_core::{Error as CoreError, EventListener};
use gadget_std::time::Duration;

#[derive(Default)]
pub struct PeriodicEventListener<const MSEC: usize, T, Event, Ctx = ()> {
    listener: T,
    _pd: gadget_std::marker::PhantomData<(Event, Ctx)>,
}

#[async_trait]
impl<
        const MSEC: usize,
        T: EventListener<Event, Ctx>,
        Ctx: Send + Sync + 'static,
        Event: Send + Sync + 'static,
    > EventListener<Event, Ctx> for PeriodicEventListener<MSEC, T, Event, Ctx>
{
    type ProcessorError = Error;

    async fn new(context: &Ctx) -> Result<Self, CoreError<Self::ProcessorError>>
    where
        Self: Sized,
    {
        let listener = T::new(context)
            .await
            .map_err(|e| Error::InnerListener(e.to_string()))?;
        Ok(Self {
            listener,
            _pd: gadget_std::marker::PhantomData,
        })
    }

    async fn next_event(&mut self) -> Option<Event> {
        let interval = Duration::from_millis(MSEC as u64);
        tokio::time::sleep(interval).await;
        self.listener.next_event().await
    }
}
