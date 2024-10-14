use std::future::Future;
use std::pin::Pin;
use crate::events_watcher::InitializableEventHandler;

pub struct MultiJobRunner {
    pub enqueued_job_runners: Vec<Pin<Box<dyn SendFuture<Output=Option<tokio::sync::oneshot::Receiver<()>>>>>>,
}

pub trait SendFuture: Send + Future + 'static {}
impl<T: Send + Future + 'static> SendFuture for T {}

impl MultiJobRunner {
    pub fn new() -> Self {
        Self {
            enqueued_job_runners: Vec::new(),
        }
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
    /// MultiJobRunner::new().with_job(x_square).with_job(x_square2).run().await;
    pub fn with_job<T: InitializableEventHandler<R>, R>(&mut self, job_runner: T) -> &mut Self
        where T: Send + 'static,
              R: Send + 'static, {
        let task = Box::pin(async move {
            job_runner.init_event_handler().await
        });

        self.enqueued_job_runners.push(task);
        self
    }

    pub async fn run(&mut self) -> Result<(), crate::Error> {
        if self.enqueued_job_runners.is_empty() {
            return Err(crate::Error::Other("No jobs registered. Make sure to add a job with `MultiJobRunner::add_job` ".to_string()));
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
            let receiver = receiver.ok_or_else(||crate::Error::Other("Job already initialized".to_string()))?;
            ordered_futures.push(receiver);
        }

        let res = futures::future::select_all(ordered_futures).await;
        Err(crate::Error::Other(format!("Job {} exited prematurely", res.1)))
    }
}