use blueprint_core::Job;
use blueprint_router::Router;
use blueprint_runner::BlueprintConfig;
use blueprint_runner::config::BlueprintEnvironment;
use blueprint_runner::error::RunnerError as Error;
use blueprint_runner::{BackgroundService, BlueprintRunner, BlueprintRunnerBuilder};
use std::future;
use std::future::Pending;

pub struct TestRunner<Ctx> {
    router: Option<Router<Ctx>>,
    context: Option<Ctx>,
    job_index: usize,
    #[doc(hidden)]
    pub builder: Option<BlueprintRunnerBuilder<Pending<()>>>,
    _phantom: core::marker::PhantomData<Ctx>,
}

impl<Ctx> TestRunner<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
{
    pub fn new<C>(config: C, env: BlueprintEnvironment, context: Ctx) -> Self
    where
        C: BlueprintConfig + 'static,
    {
        let builder =
            BlueprintRunner::builder(config, env).with_shutdown_handler(future::pending());
        TestRunner {
            router: Some(Router::new()),
            context: Some(context),
            job_index: 0,
            builder: Some(builder),
            _phantom: core::marker::PhantomData,
        }
    }

    #[expect(clippy::missing_panics_doc)]
    pub fn add_job<J, T>(&mut self, job: J) -> &mut Self
    where
        J: Job<T, Ctx> + Send + Sync + 'static,
        T: 'static,
    {
        self.router = Some(
            self.router
                .take()
                .expect("router should always exist")
                .route(self.job_index, job),
        );
        self.job_index += 1;
        self
    }

    #[expect(clippy::missing_panics_doc)]
    pub fn add_background_service<B>(&mut self, service: B) -> &mut Self
    where
        B: BackgroundService + Send + 'static,
    {
        self.builder = Some(
            self.builder
                .take()
                .expect("router should always exist")
                .background_service(service),
        );
        self
    }

    /// Start the runner
    ///
    /// # Errors
    ///
    /// See [`BlueprintRunnerBuilder::run()`]
    #[expect(clippy::missing_panics_doc)]
    pub async fn run(self) -> Result<(), Error> {
        self.builder
            .unwrap()
            .router(self.router.unwrap().with_context(self.context.unwrap()))
            .run()
            .await
    }
}

pub trait TestEnv: Sized {
    type Config: BlueprintConfig;
    type Context: Clone + Send + Sync + 'static;

    /// Create a new test environment
    ///
    /// # Errors
    ///
    /// Errors depend on the implementation.
    fn new(
        config: Self::Config,
        env: BlueprintEnvironment,
        context: Self::Context,
    ) -> Result<Self, Error>;
    fn add_job<J, T>(&mut self, job: J)
    where
        J: Job<T, Self::Context> + Send + Sync + 'static,
        T: 'static;
    fn add_background_service<B>(&mut self, service: B)
    where
        B: BackgroundService + Send + 'static;
    fn get_gadget_config(&self) -> BlueprintEnvironment;

    /// Start the runner
    ///
    /// # Panics
    ///
    /// Will panic if the runner is already started
    fn run_runner(&mut self) -> impl std::future::Future<Output = Result<(), Error>> + Send;
}
