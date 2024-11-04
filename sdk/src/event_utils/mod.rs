use async_trait::async_trait;

#[async_trait]
pub trait InitializableEventHandler {
    async fn init_event_handler(
        &self,
    ) -> Option<tokio::sync::oneshot::Receiver<Result<(), crate::Error>>>;
}
