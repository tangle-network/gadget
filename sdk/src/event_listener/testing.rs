use crate::event_listener::EventListener;
use async_trait::async_trait;

/// Only useful for testing job syntax, not running jobs.
/// This event listener will never return an event.
pub struct PendingEventListener<T, Ctx>(std::marker::PhantomData<(T, Ctx)>);

#[async_trait]
impl<T: Send + 'static, Ctx: Send + 'static> EventListener<T, Ctx>
    for PendingEventListener<T, Ctx>
{
    async fn new(_context: &Ctx) -> Result<Self, crate::Error>
    where
        Self: Sized,
    {
        Ok(Self(std::marker::PhantomData))
    }

    async fn next_event(&mut self) -> Option<T> {
        futures::future::pending().await
    }
}
