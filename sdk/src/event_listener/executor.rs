use crate::event_listener::EventListener;
use async_trait::async_trait;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;

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
        Result<Self::PreprocessedEvent, crate::Error>,
        BoxedFuture<Result<Self::PreprocessedEvent, crate::Error>>,
    >;
    type JobProcessor: ProcessorFunction<
        Self::PreprocessedEvent,
        Result<Self::JobProcessedEvent, crate::Error>,
        BoxedFuture<Result<Self::JobProcessedEvent, crate::Error>>,
    >;
    type JobProcessedEvent: Send + 'static;
    type PostProcessor: ProcessorFunction<
        Self::JobProcessedEvent,
        Result<(), crate::Error>,
        BoxedFuture<Result<(), crate::Error>>,
    >;

    fn get_preprocessor(&mut self) -> &mut Self::PreProcessor;
    fn get_job_processor(&mut self) -> &mut Self::JobProcessor;
    fn get_postprocessor(&mut self) -> &mut Self::PostProcessor;

    async fn pre_process(&mut self, event: T) -> Result<Self::PreprocessedEvent, crate::Error> {
        self.get_preprocessor()(event).await
    }

    async fn process(
        &mut self,
        preprocessed_event: Self::PreprocessedEvent,
    ) -> Result<Self::JobProcessedEvent, crate::Error> {
        self.get_job_processor()(preprocessed_event).await
    }

    async fn post_process(
        &mut self,
        job_output: Self::JobProcessedEvent,
    ) -> Result<(), crate::Error> {
        self.get_postprocessor()(job_output).await
    }

    async fn event_loop(&mut self) -> Result<(), crate::Error> {
        // TODO: add exponential backoff logic here
        while let Some(event) = self.next_event().await {
            match self.pre_process(event).await {
                Ok(preprocessed_event) => {
                    let job_output = self.process(preprocessed_event).await?;
                    self.post_process(job_output).await?;
                }
                Err(crate::Error::SkipPreProcessedType) => {}
                Err(crate::Error::BadArgumentDecoding(err)) => {
                    crate::warn!("Bad argument decoding, will skip handling event and consequentially triggering the job: {}", err);
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        Err(crate::Error::Other(
            "Event loop ended unexpectedly".to_string(),
        ))
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
> {
    event_listener: Box<dyn EventListener<Event, Ctx>>,
    preprocessor: Box<dyn Fn(Event) -> BoxedFuture<Result<PreProcessOut, crate::Error>> + Send>,
    job_processor:
        Box<dyn Fn(PreProcessOut) -> BoxedFuture<Result<JobOutput, crate::Error>> + Send>,
    postprocessor: Box<dyn Fn(JobOutput) -> BoxedFuture<Result<(), crate::Error>> + Send>,
    _pd: PhantomData<Ctx>,
}

impl<
        Ctx: Send + 'static,
        Event: Send + 'static,
        PreProcessOut: Send + 'static,
        JobOutput: Send + 'static,
    > EventFlowWrapper<Ctx, Event, PreProcessOut, JobOutput>
{
    pub fn new<T, Pre, PreFut, Job, JobFut, Post, PostFut>(
        event_listener: T,
        preprocessor: Pre,
        job_processor: Job,
        postprocessor: Post,
    ) -> Self
    where
        T: EventListener<Event, Ctx>,
        Pre: Fn(Event) -> PreFut + Send + 'static,
        PreFut: Future<Output = Result<PreProcessOut, crate::Error>> + Send + 'static,
        Job: Fn(PreProcessOut) -> JobFut + Send + 'static,
        JobFut: Future<Output = Result<JobOutput, crate::Error>> + Send + 'static,
        Post: Fn(JobOutput) -> PostFut + Send + 'static,
        PostFut: Future<Output = Result<(), crate::Error>> + Send + 'static,
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
    > EventFlowExecutor<Event, Ctx> for EventFlowWrapper<Ctx, Event, PreProcessOut, JobOutput>
{
    type PreprocessedEvent = PreProcessOut;
    type PreProcessor =
        Box<dyn Fn(Event) -> BoxedFuture<Result<Self::PreprocessedEvent, crate::Error>> + Send>;
    type JobProcessor = Box<
        dyn Fn(
                Self::PreprocessedEvent,
            ) -> BoxedFuture<Result<Self::JobProcessedEvent, crate::Error>>
            + Send,
    >;
    type JobProcessedEvent = JobOutput;
    type PostProcessor =
        Box<dyn Fn(Self::JobProcessedEvent) -> BoxedFuture<Result<(), crate::Error>> + Send>;

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
    > EventListener<Event, Ctx> for EventFlowWrapper<Ctx, Event, PreProcessOut, JobOutput>
{
    async fn new(_context: &Ctx) -> Result<Self, crate::Error> {
        unreachable!("Not called here")
    }

    async fn next_event(&mut self) -> Option<Event> {
        self.event_listener.next_event().await
    }
}

#[cfg(test)]
mod tests {
    use crate::event_listener::executor::{EventFlowExecutor, EventFlowWrapper};
    use crate::event_listener::EventListener;
    use crate::Error;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    struct DummyEventListener(Arc<AtomicU64>);

    type TestEvent = Arc<AtomicU64>;

    #[async_trait]
    impl EventListener<TestEvent, Arc<AtomicU64>> for DummyEventListener {
        async fn new(context: &Arc<AtomicU64>) -> Result<Self, Error>
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

    async fn preprocess(event: TestEvent) -> Result<(u64, TestEvent), Error> {
        let amount = event.fetch_add(1, Ordering::SeqCst) + 1;
        Ok((amount, event))
    }

    async fn job_processor(preprocessed_event: (u64, TestEvent)) -> Result<u64, Error> {
        let amount = preprocessed_event.1.fetch_add(1, Ordering::SeqCst) + 1;
        Ok(amount)
    }

    async fn post_process(_job_output: u64) -> Result<(), Error> {
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
