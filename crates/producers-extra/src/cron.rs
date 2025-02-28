use blueprint_core::{BoxError, Bytes, IntoJobId, JobCall, JobId};
use chrono::{TimeZone, Utc};
use core::pin::Pin;
use core::task::Poll;
use futures::Stream;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};

/// A producer that generates [`JobCall`]s on a [cron] schedule
///
/// Note that the schedule **must** include seconds, which is *non-standard*.
///
/// This will produce [`JobCall`]s with an empty body, meaning in order for a [`Job`] to receive any
/// calls, it is *at most* allowed to take a [`Context`].
///
/// # Usage
///
/// ```rust
/// use blueprint_producers_extra::cron::CronJob;
/// use chrono::Utc;
/// use futures::StreamExt;
/// use tokio::time::Instant;
/// use std::time::Duration;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), tokio_cron_scheduler::JobSchedulerError> {
/// const MY_JOB_ID: u8 = 0;
///
/// // I want to run my job every 5 seconds
/// let mut cron = CronJob::new(MY_JOB_ID, "5 * * * * *").await?;
///
/// // Verify that we're producing something
/// assert!(cron.next().await.is_some());
///
/// // Verify that we're producing every 10 seconds
/// let before = Instant::now();
/// let next_call = cron.next().await;
/// let after = Instant::now();
///
/// assert!(after.duration_since(before) >= Duration::from_secs(10));
/// # Ok(()) }
/// ```
///
/// [cron]: https://en.wikipedia.org/wiki/Cron
pub struct CronJob {
    job_id: JobId,
    _scheduler: JobScheduler,
    rx: UnboundedReceiver<()>,
}

impl CronJob {
    /// Create a new [`CronJob`], using the [`Utc`] timezone
    pub async fn new<I, S>(
        job_id: I,
        schedule: S,
    ) -> Result<Self, JobSchedulerError>
    where
        I: IntoJobId,
        S: AsRef<str>,
    {
        Self::new_tz(job_id, schedule, Utc)
    }

    /// Create a new [`CronJob`] with the specified `timezone`
    pub async fn new_tz<I, S, Tz>(
        job_id: I,
        schedule: S,
        timezone: Tz,
    ) -> Result<Self, JobSchedulerError>
    where
        I: IntoJobId,
        S: AsRef<str>,
        Tz: TimeZone,
    {
        let scheduler = JobScheduler::new().await?;
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        scheduler
            .add(Job::new_tz(schedule.as_ref(), timezone, move |_, _| {
                if let Err(e) = tx.send(()) {
                    #[cfg(feature = "tracing")]
                    tracing::error!("Failed to send cron job message: {e}");
                }
            })?)
            .await?;

        scheduler.start().await?;

        Ok(Self {
            job_id: job_id.into_job_id(),
            _scheduler: scheduler,
            rx,
        })
    }
}

impl Stream for CronJob {
    type Item = Result<JobCall, BoxError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> Poll<Option<Self::Item>> {
        match self.as_mut().rx.poll_recv(cx) {
            Poll::Ready(Some(())) => Poll::Ready(Some(Ok(JobCall::new(self.job_id, Bytes::new())))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::cron::CronJob;
    use blueprint_core::JobId;
    use futures::StreamExt;
    use std::time::Duration;
    use tokio::time::Instant;

    #[tokio::test]
    async fn it_works() {
        let mut cron = CronJob::new(0, "* * * * * *").await.unwrap();

        // Should continuously return
        for _ in 0..10 {
            let before = Instant::now();
            let j = cron.next().await.unwrap();
            let after = Instant::now();

            // The task should be running every second, but give it 500ms of freedom
            assert!(after.duration_since(before) < Duration::from_millis(1500));

            assert!(j.is_ok());
            assert_eq!(j.unwrap().job_id(), JobId::from(0));
        }
    }
}
