use async_trait::async_trait;
use tokio::sync::MutexGuard;
use tokio::time::error::Elapsed;

#[async_trait]
pub trait TokioMutexExt<T: Send> {
    async fn try_lock_timeout(
        &self,
        timeout: std::time::Duration,
    ) -> Result<MutexGuard<T>, Elapsed>;
    async fn lock_timeout(&self, timeout: std::time::Duration) -> MutexGuard<T> {
        self.try_lock_timeout(timeout)
            .await
            .expect("Timeout on mutex lock")
    }
}

#[async_trait]
impl<T: Send> TokioMutexExt<T> for tokio::sync::Mutex<T> {
    async fn try_lock_timeout(
        &self,
        timeout: std::time::Duration,
    ) -> Result<MutexGuard<T>, Elapsed> {
        tokio::time::timeout(timeout, self.lock()).await
    }
}
