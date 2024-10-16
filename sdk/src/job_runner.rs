use crate::config::GadgetConfiguration;
use crate::events_watcher::InitializableEventHandler;
use std::future::Future;
use std::pin::Pin;

pub struct MultiJobRunner {
    pub(crate) enqueued_job_runners: EnqueuedJobRunners,
    pub(crate) test_mode: bool,
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

pub struct JobBuilder<'a, T> {
    pub(crate) register_call: Option<RegisterCall>,
    context: Option<T>,
    runner: &'a mut MultiJobRunner,
}

pub(crate) type RegisterCall = Pin<Box<dyn SendFuture<Output = Result<(), crate::Error>>>>;

impl<T> JobBuilder<'_, T> {
    pub fn with_registration<Fut: SendFuture<Output = Result<(), crate::Error>>>(
        &mut self,
        register_call: fn(T) -> Fut,
    ) -> &mut Self {
        let context = self
            .context
            .take()
            .expect("Cannot call this function twice");
        let future = register_call(context);
        self.register_call = Some(Box::pin(future));
        self
    }

    pub fn add_job<K: InitializableEventHandler + Send + 'static>(
        &mut self,
        job_runner: K,
    ) -> &mut MultiJobRunner {
        let registration = self.register_call.take();
        let test_mode = self.runner.test_mode;

        let task = Box::pin(async move {
            if let Some(registration) = registration {
                // Skip registration if in test mode
                if !test_mode {
                    if let Err(err) = registration.await {
                        crate::error!("Failed to register job: {err:?}");
                        return None;
                    }
                }
            }

            job_runner.init_event_handler().await
        });
        self.runner.enqueued_job_runners.push(task);

        self.runner
    }
}

impl MultiJobRunner {
    pub fn new(env: &GadgetConfiguration<parking_lot::RawRwLock>) -> Self {
        Self {
            enqueued_job_runners: Vec::new(),
            test_mode: env.test_mode,
        }
    }

    pub fn with_job_context<'a, 'b: 'a, T: 'b>(&'a mut self, context: T) -> JobBuilder<'a, T> {
        JobBuilder {
            register_call: None,
            runner: self,
            context: Some(context),
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
    ///
    /// MultiJobRunner::new().with_job().add_job(x_square).with_job().add_job(x_square2).run().await;
    pub fn with_job(&mut self) -> JobBuilder<'_, ()> {
        self.with_job_context(())
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
