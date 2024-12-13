use gadget_std::time::Duration;
use tokio::{sync::MutexGuard, time::error::Elapsed};

/// An extension trait for tokio Mutex.
///
/// This allows for locking a mutex with a given timeout.
#[async_trait::async_trait]
#[allow(clippy::module_name_repetitions)]
pub trait TokioMutexExt<T: Send> {
    /// Attempts to lock the mutex with a given timeout.
    ///
    /// # Errors
    ///
    /// Returns an error if the timeout expires before the lock is acquired.
    async fn try_lock_timeout(&self, timeout: Duration) -> Result<MutexGuard<'_, T>, Elapsed>;

    /// Locks the mutex with a given timeout.
    ///
    /// # Panics
    ///
    /// Panics if [try_lock_timeout] returns an error.
    ///
    /// [try_lock_timeout]: Self::try_lock_timeout
    async fn lock_timeout(&self, timeout: Duration) -> MutexGuard<'_, T> {
        self.try_lock_timeout(timeout)
            .await
            .expect("Timeout on mutex lock")
    }
}

#[async_trait::async_trait]
impl<T: Send> TokioMutexExt<T> for tokio::sync::Mutex<T> {
    async fn try_lock_timeout(&self, timeout: Duration) -> Result<MutexGuard<'_, T>, Elapsed> {
        tokio::time::timeout(timeout, self.lock()).await
    }
}
