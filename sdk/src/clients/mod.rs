use async_trait::async_trait;
use auto_impl::auto_impl;

pub mod tangle;

#[async_trait]
#[auto_impl(Arc)]
pub trait Client<Event>: Clone + Send + Sync {
    async fn next_event(&self) -> Option<Event>;
    async fn latest_event(&self) -> Option<Event>;
}
