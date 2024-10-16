use crate::events_watcher::InitializableEventHandler;
use std::future::Future;
use std::pin::Pin;

#[derive(Default)]
pub struct MultiJobRunner {
    pub(crate) enqueued_job_runners: EnqueuedJobRunners,
}

pub type EnqueuedJobRunners = Vec<
    Pin<
        Box<
            dyn SendFuture<
                Output = Option<tokio::sync::oneshot::Receiver<Result<(), crate::Error>>>,
            >,
        >,
    >,
>;

pub trait SendFuture: Send + Future + 'static {}
impl<T: Send + Future + 'static> SendFuture for T {}

impl MultiJobRunner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a job to the job runner
    ///
    /// let x_square = blueprint::XsquareEventHandler {
    ///      service_id: self.env.service_id.unwrap(),
    ///      client: client.clone(),
    ///      signer: signer.clone(),
    /// };
    ///
    /// let x_square2 = blueprint::XsquareEventHandler {
    ///      service_id: self.env.service_id.unwrap(),
    ///      client: client.clone(),
    ///      signer,
    /// };
    ///
    /// MultiJobRunner::new().with_job(x_square).with_job(x_square2).run().await;
    pub fn with_job<T: InitializableEventHandler + Send + 'static>(
        &mut self,
        job_runner: T,
    ) -> &mut Self {
        let task = Box::pin(async move { job_runner.init_event_handler().await });

        self.enqueued_job_runners.push(task);
        self
    }

    pub async fn run(&mut self) -> Result<(), crate::Error> {
        if self.enqueued_job_runners.is_empty() {
            return Err(crate::Error::Other(
                "No jobs registered. Make sure to add a job with `MultiJobRunner::add_job` "
                    .to_string(),
            ));
        }

        let mut futures = Vec::new();
        let tasks = std::mem::take(&mut self.enqueued_job_runners);
        for job_runner in tasks {
            futures.push(job_runner);
        }

        let receivers = futures::future::join_all(futures).await;

        // For each receiver, take the value out of it and add it to an ordered futures list. If any values
        // is None, return an error stating that the job already initialized
        let mut ordered_futures = Vec::new();
        for receiver in receivers {
            let receiver = receiver
                .ok_or_else(|| crate::Error::Other("Job already initialized".to_string()))?;
            ordered_futures.push(receiver);
        }

        let res = futures::future::select_all(ordered_futures).await;
        let job_n = res.1;
        let err = res
            .0
            .map_err(|err| crate::Error::Other(err.to_string()))
            .map_err(|_err| {
                crate::Error::Other(format!("Job {job_n} exited prematurely (channel dropped)"))
            })?;

        Err(crate::Error::Other(format!(
            "Job {job_n} exited prematurely: {err:?}"
        )))
    }
}
