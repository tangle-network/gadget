use async_trait::async_trait;
use std::time::Duration;
use tokio::sync::MutexGuard;
use tokio::time;

#[async_trait]
pub trait TokioMutexExt<T: Send> {
    async fn try_lock_timeout(
        &self,
        timeout: Duration,
    ) -> Result<MutexGuard<T>, time::error::Elapsed>;
    async fn lock_timeout(&self, timeout: Duration) -> MutexGuard<T> {
        self.try_lock_timeout(timeout)
            .await
            .expect("Timeout on mutex lock")
    }
}

#[async_trait]
impl<T: Send> TokioMutexExt<T> for tokio::sync::Mutex<T> {
    async fn try_lock_timeout(
        &self,
        timeout: Duration,
    ) -> Result<MutexGuard<T>, time::error::Elapsed> {
        time::timeout(timeout, self.lock()).await
    }
}
