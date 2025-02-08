use crate::error::Error;
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
pub trait EventFlowExecutor<T, Ctx, Creator>
where
    T: Send + 'static,
    Ctx: Clone + Send + 'static,
    Creator: Send + 'static,
    Self: EventListener<T, Ctx, Creator>,
{
    type PreprocessedEvent: Send + 'static;
    type PreProcessor: ProcessorFunction<
        T,
        Result<
            Option<Self::PreprocessedEvent>,
            Error<<Self as EventListener<T, Ctx, Creator>>::ProcessorError>,
        >,
        BoxedFuture<
            Result<
                Option<Self::PreprocessedEvent>,
                Error<<Self as EventListener<T, Ctx, Creator>>::ProcessorError>,
            >,
        >,
    >;
    type JobProcessor: ProcessorFunction<
        (Self::PreprocessedEvent, Ctx),
        Result<
            Self::JobProcessedEvent,
            Error<<Self as EventListener<T, Ctx, Creator>>::ProcessorError>,
        >,
        BoxedFuture<
            Result<
                Self::JobProcessedEvent,
                Error<<Self as EventListener<T, Ctx, Creator>>::ProcessorError>,
            >,
        >,
    >;
    type JobProcessedEvent: Send + 'static;
    type PostProcessor: ProcessorFunction<
        Self::JobProcessedEvent,
        Result<(), Error<<Self as EventListener<T, Ctx, Creator>>::ProcessorError>>,
        BoxedFuture<Result<(), Error<<Self as EventListener<T, Ctx, Creator>>::ProcessorError>>>,
    >;
    fn get_context(&self) -> &Ctx;

    fn get_preprocessor(&mut self) -> &mut Self::PreProcessor;
    fn get_job_processor(&mut self) -> &mut Self::JobProcessor;
    fn get_postprocessor(&mut self) -> &mut Self::PostProcessor;

    async fn pre_process(
        &mut self,
        event: T,
    ) -> Result<
        Option<Self::PreprocessedEvent>,
        Error<<Self as EventListener<T, Ctx, Creator>>::ProcessorError>,
    > {
        self.get_preprocessor()(event).await
    }

    async fn process(
        &mut self,
        preprocessed_event: Self::PreprocessedEvent,
    ) -> Result<
        Self::JobProcessedEvent,
        Error<<Self as EventListener<T, Ctx, Creator>>::ProcessorError>,
    > {
        let context = self.get_context().clone();
        self.get_job_processor()((preprocessed_event, context)).await
    }

    async fn post_process(
        &mut self,
        job_output: Self::JobProcessedEvent,
    ) -> Result<(), Error<<Self as EventListener<T, Ctx, Creator>>::ProcessorError>> {
        self.get_postprocessor()(job_output).await
    }

    async fn event_loop(
        &mut self,
    ) -> Result<(), Error<<Self as EventListener<T, Ctx, Creator>>::ProcessorError>> {
        // TODO: add exponential backoff logic here
        while let Some(event) = self.next_event().await {
            match self.pre_process(event).await {
                Ok(Some(preprocessed_event)) => {
                    let job_output = self.process(preprocessed_event).await?;
                    self.post_process(job_output).await?;
                }
                // Skipped
                Ok(None) => {}
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

pub trait ProcessorFunction<Event, Out, Fut>: FnMut(Event) -> Fut
where
    Fut: Future<Output = Out> + Send + 'static,
    Event: Send + 'static,
    Out: Send + 'static,
{
}

// Blanket implementation of ProcessorFunction for all functions that satisfy the constraints
impl<F, Event, Out, Fut> ProcessorFunction<Event, Out, Fut> for F
where
    F: FnMut(Event) -> Fut,
    Fut: Future<Output = Out> + Send + 'static,
    Event: Send + 'static,
    Out: Send + 'static,
{
}

pub type BoxedFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

#[allow(clippy::type_complexity)]
pub struct EventFlowWrapper<
    Ctx: Clone + Send + 'static,
    Creator: Send + 'static,
    Event: Send + 'static,
    PreProcessOut: Send + 'static,
    JobOutput: Send + 'static,
    ProcessorError: core::error::Error + Send + Sync + 'static,
> {
    context: Ctx,
    event_listener: Box<dyn EventListener<Event, Ctx, Creator, ProcessorError = ProcessorError>>,
    preprocessor: Box<
        dyn Fn(Event) -> BoxedFuture<Result<Option<PreProcessOut>, Error<ProcessorError>>> + Send,
    >,
    job_processor: Box<
        dyn Fn((PreProcessOut, Ctx)) -> BoxedFuture<Result<JobOutput, Error<ProcessorError>>>
            + Send,
    >,
    postprocessor:
        Box<dyn FnMut(JobOutput) -> BoxedFuture<Result<(), Error<ProcessorError>>> + Send>,
    _pd: PhantomData<Ctx>,
}

/*
pub struct EventFlowFilter<O: Send + 'static> {
    filte
}*/

impl<
        Ctx: Clone + Send + 'static,
        Event: Send + 'static,
        Creator: Send + 'static,
        PreProcessOut: Send + 'static,
        JobOutput: Send + 'static,
        ProcessorError: core::error::Error + Send + Sync + 'static,
    > EventFlowWrapper<Ctx, Creator, Event, PreProcessOut, JobOutput, ProcessorError>
{
    pub fn new<T, Pre, PreFut, Job, JobFut, Post, PostFut>(
        context: Ctx,
        event_listener: T,
        preprocessor: Pre,
        job_processor: Job,
        mut postprocessor: Post,
    ) -> Self
    where
        T: EventListener<Event, Ctx, Creator, ProcessorError = ProcessorError>,
        Pre: Fn(Event) -> PreFut + Send + 'static,
        PreFut:
            Future<Output = Result<Option<PreProcessOut>, Error<ProcessorError>>> + Send + 'static,
        Job: Fn((PreProcessOut, Ctx)) -> JobFut + Send + 'static,
        JobFut: Future<Output = Result<JobOutput, Error<ProcessorError>>> + Send + 'static,
        Post: FnMut(JobOutput) -> PostFut + Send + 'static,
        PostFut: Future<Output = Result<(), Error<ProcessorError>>> + Send + 'static,
    {
        Self {
            context,
            event_listener: Box::new(event_listener),
            preprocessor: Box::new(move |event| Box::pin(preprocessor(event))),
            job_processor: Box::new(move |(event, ctx)| Box::pin(job_processor((event, ctx)))),
            postprocessor: Box::new(move |event| Box::pin(postprocessor(event))),
            _pd: PhantomData,
        }
    }
}

#[async_trait]
impl<
        Ctx: Clone + Send + 'static,
        Creator: Send + 'static,
        Event: Send + 'static,
        PreProcessOut: Send + 'static,
        JobOutput: Send + 'static,
        ProcessorError: core::error::Error + Send + Sync + 'static,
    > EventFlowExecutor<Event, Ctx, Creator>
    for EventFlowWrapper<Ctx, Creator, Event, PreProcessOut, JobOutput, ProcessorError>
{
    type PreprocessedEvent = PreProcessOut;
    type PreProcessor = Box<
        dyn Fn(Event) -> BoxedFuture<Result<Option<Self::PreprocessedEvent>, Error<ProcessorError>>>
            + Send,
    >;
    type JobProcessor = Box<
        dyn Fn(
                (Self::PreprocessedEvent, Ctx),
            ) -> BoxedFuture<Result<Self::JobProcessedEvent, Error<ProcessorError>>>
            + Send,
    >;
    type JobProcessedEvent = JobOutput;
    type PostProcessor = Box<
        dyn FnMut(Self::JobProcessedEvent) -> BoxedFuture<Result<(), Error<ProcessorError>>> + Send,
    >;

    fn get_context(&self) -> &Ctx {
        &self.context
    }

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
        Ctx: Clone + Send + 'static,
        Creator: Send + 'static,
        Event: Send + 'static,
        PreProcessOut: Send + 'static,
        JobOutput: Send + 'static,
        ProcessorError: core::error::Error + Send + Sync + 'static,
    > EventListener<Event, Ctx, Creator>
    for EventFlowWrapper<Ctx, Creator, Event, PreProcessOut, JobOutput, ProcessorError>
{
    type ProcessorError = ProcessorError;

    async fn new(_context: &Creator) -> Result<Self, Error<Self::ProcessorError>> {
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
    use gadget_std::convert::Infallible;
    use gadget_std::sync::atomic::{AtomicU64, Ordering};
    use gadget_std::sync::Arc;
    use gadget_std::time::Duration;

    struct DummyEventListener(Arc<AtomicU64>);

    type TestEvent = Arc<AtomicU64>;

    #[async_trait]
    impl EventListener<TestEvent, Arc<AtomicU64>> for DummyEventListener {
        type ProcessorError = Infallible;

        async fn new(context: &Arc<AtomicU64>) -> Result<Self, Error<Self::ProcessorError>>
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

    async fn preprocess(event: TestEvent) -> Result<Option<(u64, TestEvent)>, Error<Infallible>> {
        let amount = event.fetch_add(1, Ordering::SeqCst) + 1;
        Ok(Some((amount, event)))
    }

    async fn job_processor(
        preprocessed_event: ((u64, TestEvent), Arc<AtomicU64>),
    ) -> Result<u64, Error<Infallible>> {
        let amount = preprocessed_event.1.fetch_add(1, Ordering::SeqCst) + 1;
        Ok(amount)
    }

    async fn post_process(_job_output: u64) -> Result<(), Error<Infallible>> {
        Ok(())
    }

    #[tokio::test]
    async fn test_event_flow_executor_builds() {
        let counter = Arc::new(AtomicU64::new(0));
        let _event_listener = EventFlowWrapper::new(
            Box::new(|_| ()),
            counter.clone(),
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
            Box::new(|_| ()),
            counter.clone(),
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
