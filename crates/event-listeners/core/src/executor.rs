use crate::error::{Error, Result};
use crate::EventListener;
use async_trait::async_trait;
use gadget_std::future::Future;
use gadget_std::marker::PhantomData;
use gadget_std::pin::Pin;

/// [`EventFlowExecutor`]: Allows flexible and organized execution of events
///
/// Provides the structure for the workflow of taking events and running them through a series of steps.
/// This is meant to be auto-implemented by the job macro onto the provided structs that implement T: EventListener.
#[async_trait]
pub trait EventFlowExecutor<T, Ctx>
where
    T: Send + 'static,
    Ctx: Send + 'static,
    Self: EventListener<T, Ctx>,
{
    type PreprocessedEvent: Send + 'static;
    type PreProcessor: ProcessorFunction<
        T,
        Result<Self::PreprocessedEvent>,
        BoxedFuture<Result<Self::PreprocessedEvent>>,
    >;
    type JobProcessor: ProcessorFunction<
        Self::PreprocessedEvent,
        Result<Self::JobProcessedEvent>,
        BoxedFuture<Result<Self::JobProcessedEvent>>,
    >;
    type JobProcessedEvent: Send + 'static;
    type PostProcessor: ProcessorFunction<
        Self::JobProcessedEvent,
        Result<()>,
        BoxedFuture<Result<()>>,
    >;

    fn get_preprocessor(&mut self) -> &mut Self::PreProcessor;
    fn get_job_processor(&mut self) -> &mut Self::JobProcessor;
    fn get_postprocessor(&mut self) -> &mut Self::PostProcessor;

    async fn pre_process(&mut self, event: T) -> Result<Self::PreprocessedEvent> {
        self.get_preprocessor()(event).await
    }

    async fn process(
        &mut self,
        preprocessed_event: Self::PreprocessedEvent,
    ) -> Result<Self::JobProcessedEvent> {
        self.get_job_processor()(preprocessed_event).await
    }

    async fn post_process(&mut self, job_output: Self::JobProcessedEvent) -> Result<()> {
        self.get_postprocessor()(job_output).await
    }

    async fn event_loop(&mut self) -> Result<()> {
        // TODO: add exponential backoff logic here
        while let Some(event) = self.next_event().await {
            match self.pre_process(event).await {
                Ok(preprocessed_event) => {
                    let job_output = self.process(preprocessed_event).await?;
                    self.post_process(job_output).await?;
                }
                Err(Error::SkipPreProcessedType) => {}
                Err(Error::BadArgumentDecoding(err)) => {
                    gadget_logging::warn!("Bad argument decoding, will skip handling event and consequentially triggering the job: {}", err);
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        Err(Error::Termination)
    }
}

pub trait ProcessorFunction<Event, Out, Fut>: Fn(Event) -> Fut
where
    Fut: Future<Output = Out> + Send + 'static,
    Event: Send + 'static,
    Out: Send + 'static,
{
}

// Blanket implementation of ProcessorFunction for all functions that satisfy the constraints
impl<F, Event, Out, Fut> ProcessorFunction<Event, Out, Fut> for F
where
    F: Fn(Event) -> Fut,
    Fut: Future<Output = Out> + Send + 'static,
    Event: Send + 'static,
    Out: Send + 'static,
{
}

pub type BoxedFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

pub struct EventFlowWrapper<
    Ctx: Send + 'static,
    Event: Send + 'static,
    PreProcessOut: Send + 'static,
    JobOutput: Send + 'static,
    Error: core::error::Error + Send + Sync + 'static,
> {
    event_listener: Box<dyn EventListener<Event, Ctx, Error = Error>>,
    preprocessor: Box<dyn Fn(Event) -> BoxedFuture<Result<PreProcessOut>> + Send>,
    job_processor: Box<dyn Fn(PreProcessOut) -> BoxedFuture<Result<JobOutput>> + Send>,
    postprocessor: Box<dyn Fn(JobOutput) -> BoxedFuture<Result<()>> + Send>,
    _pd: PhantomData<Ctx>,
}

impl<
        Ctx: Send + 'static,
        Event: Send + 'static,
        PreProcessOut: Send + 'static,
        JobOutput: Send + 'static,
        Error: core::error::Error + Send + Sync + 'static,
    > EventFlowWrapper<Ctx, Event, PreProcessOut, JobOutput, Error>
{
    pub fn new<T, Pre, PreFut, Job, JobFut, Post, PostFut>(
        event_listener: T,
        preprocessor: Pre,
        job_processor: Job,
        postprocessor: Post,
    ) -> Self
    where
        T: EventListener<Event, Ctx, Error = Error>,
        Pre: Fn(Event) -> PreFut + Send + 'static,
        PreFut: Future<Output = Result<PreProcessOut>> + Send + 'static,
        Job: Fn(PreProcessOut) -> JobFut + Send + 'static,
        JobFut: Future<Output = Result<JobOutput>> + Send + 'static,
        Post: Fn(JobOutput) -> PostFut + Send + 'static,
        PostFut: Future<Output = Result<()>> + Send + 'static,
    {
        Self {
            event_listener: Box::new(event_listener),
            preprocessor: Box::new(move |event| Box::pin(preprocessor(event))),
            job_processor: Box::new(move |event| Box::pin(job_processor(event))),
            postprocessor: Box::new(move |event| Box::pin(postprocessor(event))),
            _pd: PhantomData,
        }
    }
}

#[async_trait]
impl<
        Ctx: Send + 'static,
        Event: Send + 'static,
        PreProcessOut: Send + 'static,
        JobOutput: Send + 'static,
        Error: core::error::Error + Send + Sync + 'static,
    > EventFlowExecutor<Event, Ctx>
    for EventFlowWrapper<Ctx, Event, PreProcessOut, JobOutput, Error>
{
    type PreprocessedEvent = PreProcessOut;
    type PreProcessor = Box<dyn Fn(Event) -> BoxedFuture<Result<Self::PreprocessedEvent>> + Send>;
    type JobProcessor =
        Box<dyn Fn(Self::PreprocessedEvent) -> BoxedFuture<Result<Self::JobProcessedEvent>> + Send>;
    type JobProcessedEvent = JobOutput;
    type PostProcessor = Box<dyn Fn(Self::JobProcessedEvent) -> BoxedFuture<Result<()>> + Send>;

    fn get_preprocessor(&mut self) -> &mut Self::PreProcessor {
        &mut self.preprocessor
    }

    fn get_job_processor(&mut self) -> &mut Self::JobProcessor {
        &mut self.job_processor
    }

    fn get_postprocessor(&mut self) -> &mut Self::PostProcessor {
        &mut self.postprocessor
    }
}

#[async_trait]
impl<
        Ctx: Send + 'static,
        Event: Send + 'static,
        PreProcessOut: Send + 'static,
        JobOutput: Send + 'static,
        Error: core::error::Error + Send + Sync + 'static,
    > EventListener<Event, Ctx> for EventFlowWrapper<Ctx, Event, PreProcessOut, JobOutput, Error>
{
    type Error = Error;

    async fn new(_context: &Ctx) -> gadget_std::result::Result<Self, Self::Error> {
        unreachable!("Not called here")
    }

    async fn next_event(&mut self) -> Option<Event> {
        self.event_listener.next_event().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use gadget_std::sync::atomic::{AtomicU64, Ordering};
    use gadget_std::sync::Arc;
    use gadget_std::time::Duration;

    struct DummyEventListener(Arc<AtomicU64>);

    type TestEvent = Arc<AtomicU64>;

    #[async_trait]
    impl EventListener<TestEvent, Arc<AtomicU64>> for DummyEventListener {
        type Error = Error;

        async fn new(context: &Arc<AtomicU64>) -> gadget_std::result::Result<Self, Self::Error>
        where
            Self: Sized,
        {
            Ok(Self(context.clone()))
        }

        async fn next_event(&mut self) -> Option<TestEvent> {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            Some(self.0.clone())
        }
    }

    async fn preprocess(event: TestEvent) -> Result<(u64, TestEvent)> {
        let amount = event.fetch_add(1, Ordering::SeqCst) + 1;
        Ok((amount, event))
    }

    async fn job_processor(preprocessed_event: (u64, TestEvent)) -> Result<u64> {
        let amount = preprocessed_event.1.fetch_add(1, Ordering::SeqCst) + 1;
        Ok(amount)
    }

    async fn post_process(_job_output: u64) -> Result<()> {
        Ok(())
    }

    #[tokio::test]
    async fn test_event_flow_executor_builds() {
        let counter = Arc::new(AtomicU64::new(0));
        let _event_listener = EventFlowWrapper::new(
            DummyEventListener(counter.clone()),
            preprocess,
            job_processor,
            post_process,
        );
    }

    #[tokio::test]
    async fn test_event_flow_executor_executes() {
        let counter = &Arc::new(AtomicU64::new(0));
        let mut event_listener = EventFlowWrapper::new(
            DummyEventListener(counter.clone()),
            preprocess,
            job_processor,
            post_process,
        );

        let executor = async move { event_listener.event_loop().await.expect("Executor failed") };
        let poller = async move {
            loop {
                tokio::time::sleep(Duration::from_millis(100)).await;
                if counter.load(Ordering::SeqCst) >= 2 {
                    break;
                }
            }
        };

        tokio::select! {
            _res0 = executor => {
                panic!("Executor failed")
            },

            _res1 = poller => {
                assert_eq!(counter.load(Ordering::SeqCst), 2);
            }
        }
    }
}
