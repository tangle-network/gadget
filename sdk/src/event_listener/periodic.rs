use crate::event_listener::EventListener;
use async_trait::async_trait;
use std::time::Duration;
use tokio::time::Instant;

#[derive(Default)]
pub struct PeriodicEventListener<const MSEC: usize, T, Evt>(std::marker::PhantomData<(T, Evt)>);

#[async_trait]
impl<const MSEC: usize, T: EventListener<Evt>, Evt: Send + Sync + 'static> EventListener<Instant>
    for PeriodicEventListener<MSEC, T, Evt>
{
    async fn next_event(&mut self) -> Option<Instant> {
        let interval = Duration::from_millis(MSEC as u64);
        tokio::time::sleep(interval).await;
        Some(Instant::now())
    }

    async fn handle_event(&mut self, event: Instant) -> std::io::Result<()> {
        crate::info!("Event at {event:?}");
        Ok(())
    }
}
