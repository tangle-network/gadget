use async_trait::async_trait;
use gadget_io::time::error::Elapsed;
use gadget_io::tokio::sync::MutexGuard;
use std::time::Duration;

#[async_trait]
pub trait TokioMutexExt<T: Send> {
    async fn try_lock_timeout(&self, timeout: Duration) -> Result<MutexGuard<'_, T>, Elapsed>;
    async fn lock_timeout(&self, timeout: Duration) -> MutexGuard<'_, T> {
        self.try_lock_timeout(timeout)
            .await
            .expect("Timeout on mutex lock")
    }
}

#[async_trait]
impl<T: Send> TokioMutexExt<T> for gadget_io::tokio::sync::Mutex<T> {
    async fn try_lock_timeout(&self, timeout: Duration) -> Result<MutexGuard<'_, T>, Elapsed> {
        gadget_io::time::timeout(timeout, self.lock()).await
    }
}
