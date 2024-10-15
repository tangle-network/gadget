use crate::event_listener::EventListener;
use crate::Error;
use async_trait::async_trait;
use std::time::Duration;

#[derive(Default)]
pub struct PeriodicEventListener<const MSEC: usize, T, Event, Ctx = ()> {
    listener: T,
    _pd: std::marker::PhantomData<(Event, Ctx)>,
}

#[async_trait]
impl<
        const MSEC: usize,
        T: EventListener<Event, Ctx>,
        Ctx: Send + Sync + 'static,
        Event: Send + Sync + 'static,
    > EventListener<Event, Ctx> for PeriodicEventListener<MSEC, T, Event, Ctx>
{
    async fn new(context: &Ctx) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let listener = T::new(context).await?;
        Ok(Self {
            listener,
            _pd: std::marker::PhantomData,
        })
    }

    async fn next_event(&mut self) -> Option<Event> {
        let interval = Duration::from_millis(MSEC as u64);
        tokio::time::sleep(interval).await;
        self.listener.next_event().await
    }

    async fn handle_event(&mut self, event: Event) -> Result<(), Error> {
        crate::info!("Event at after {MSEC}ms time received");
        self.listener.handle_event(event).await
    }
}
