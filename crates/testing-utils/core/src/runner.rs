use blueprint_core::JobCall;
use std::future;
use std::future::Pending;
use blueprint_runner::BlueprintConfig;
use blueprint_runner::config::BlueprintEnvironment;
use blueprint_runner::error::RunnerError as Error;
use blueprint_runner::{BackgroundService, BlueprintRunner, BlueprintRunnerBuilder};
use blueprint_router::Router;
use blueprint_core::Job;

pub struct TestRunner<Ctx> {
    router: Router,
    job_index: usize,
    builder: BlueprintRunnerBuilder<Pending<()>>,
    _phantom: core::marker::PhantomData<Ctx>,
}

impl<Ctx> TestRunner<Ctx> where Ctx: Clone + Send + Sync + 'static{
    pub fn new<C>(config: C, env: BlueprintEnvironment, context: Ctx) -> Self
    where
        C: BlueprintConfig + 'static,
    {
        let builder = BlueprintRunner::builder(config, env).with_shutdown_handler(future::pending());
        TestRunner {
            router: Router::new().with_context(context),
            job_index: 0,
            builder,
            _phantom: core::marker::PhantomData,
        }
    }

    pub fn add_job<J>(&mut self, job: J) -> &mut Self
    where
        J: Job<JobCall, Ctx> + Send + Sync + 'static,
    {
        self.router = self.router.route(self.job_index, job);
        self.job_index += 1;
        self
    }

    pub fn add_background_service<B>(&mut self, service: B) -> &mut Self
    where
        B: BackgroundService + Send + 'static,
    {
        self.builder.background_service(service);
        self
    }

    pub async fn run(self) -> Result<(), Error> {
        self.builder.router(self.router).run().await
    }
}

pub trait TestEnv: Sized {
    type Config: BlueprintConfig;

    fn new(config: Self::Config, env: BlueprintEnvironment) -> Result<Self, Error>;
    fn add_job<J>(&mut self, job: J)
    where
        J: Send + Sync + 'static;
    fn add_background_service<B>(&mut self, service: B)
    where
        B: BackgroundService + Send + 'static;
    fn get_gadget_config(&self) -> BlueprintEnvironment;
    fn run_runner(&self) -> impl std::future::Future<Output = Result<(), Error>> + Send;
}
