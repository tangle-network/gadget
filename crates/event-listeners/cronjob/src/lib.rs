pub mod error;

use error::Error;
use std::marker::PhantomData;
use std::sync::Arc;

use async_trait::async_trait;
use gadget_event_listeners_core::{Error as CoreError, EventListener};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};

#[derive(Clone)]
/// Used for executing the specifiied job periodically using cron notation
///
/// e.g., 1/10 * * * * *
///
/// The context must contain information about the periodicity in cron notation. See:
/// [`CronJobDefinition`]
pub struct CronJob<Ctx>(
    PhantomData<Ctx>,
    Arc<tokio::sync::Mutex<UnboundedReceiver<()>>>,
);

pub trait CronJobDefinition: Send + Sync + 'static {
    fn cron(&self) -> impl Into<String>;
}

#[async_trait]
impl<Ctx: CronJobDefinition> EventListener<(), Ctx> for CronJob<Ctx> {
    type ProcessorError = Error;

    async fn new(context: &Ctx) -> Result<Self, CoreError<Self::ProcessorError>>
    where
        Self: Sized,
    {
        let sched = JobScheduler::new().await.map_err(err_map)?;
        let cron_syntax = context.cron().into();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        // Add basic cron job
        sched
            .add(
                Job::new(cron_syntax, move |_uuid, _l| {
                    if let Err(err) = tx.send(()) {
                        gadget_logging::error!("Failed to send event to cronjob worker: {err}");
                    }
                })
                .map_err(err_map)?,
            )
            .await
            .map_err(err_map)?;

        let task = async move {
            if let Err(err) = sched.start().await {
                gadget_logging::error!("Cronjoin failed (fatal): {err}");
            }
        };

        drop(tokio::spawn(task));

        Ok(Self(PhantomData, Arc::new(Mutex::new(rx))))
    }

    async fn next_event(&mut self) -> Option<()> {
        self.1.lock().await.recv().await
    }
}

fn err_map(err: JobSchedulerError) -> CoreError<Error> {
    CoreError::EventHandler(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestContext(&'static str);

    impl CronJobDefinition for TestContext {
        fn cron(&self) -> impl Into<String> {
            self.0
        }
    }

    #[tokio::test]
    async fn cronjob_event_listener() {
        // Run every second
        let cron_job = TestContext("1/2 * * * * *");
        let mut cronjob = CronJob::new(&cron_job).await.unwrap();
        let next_event = cronjob.next_event().await;
        assert!(next_event.is_some());
    }
}
