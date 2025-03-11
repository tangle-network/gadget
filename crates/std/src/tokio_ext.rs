use crate::time::Duration;
use tokio::{sync::MutexGuard, time::error::Elapsed};

/// An extension trait for tokio Mutex.
///
/// This allows for locking a mutex with a given timeout.
pub trait TokioMutexExt<T: Send> {
    /// Attempts to lock the mutex with a given timeout.
    ///
    /// # Errors
    ///
    /// Returns an error if the timeout expires before the lock is acquired.
    async fn try_lock_timeout<'a>(
        &'a self,
        timeout: Duration,
    ) -> Result<MutexGuard<'a, T>, Elapsed>
    where
        T: 'a;

    /// Locks the mutex with a given timeout.
    ///
    /// # Panics
    ///
    /// Panics if [`try_lock_timeout`] returns an error.
    ///
    /// [try_lock_timeout]: Self::try_lock_timeout
    async fn lock_timeout<'a>(&'a self, timeout: Duration) -> MutexGuard<'a, T>
    where
        T: 'a,
    {
        self.try_lock_timeout(timeout)
            .await
            .expect("Timeout on mutex lock")
    }
}

impl<T: Send> TokioMutexExt<T> for tokio::sync::Mutex<T> {
    async fn try_lock_timeout<'a>(&'a self, timeout: Duration) -> Result<MutexGuard<'a, T>, Elapsed>
    where
        T: 'a,
    {
        tokio::time::timeout(timeout, self.lock()).await
    }
}
