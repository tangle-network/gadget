use gadget_config::GadgetConfiguration;
use gadget_event_listeners::core::InitializableEventHandler;
use gadget_runners::core::config::BlueprintConfig;
use gadget_runners::core::error::RunnerError as Error;
use gadget_runners::core::runner::{BackgroundService, BlueprintRunner};

#[derive(Clone)]
pub struct TestRunner {
    pub inner: BlueprintRunner,
}

impl TestRunner {
    pub fn new<C>(config: C, env: GadgetConfiguration) -> Self
    where
        C: BlueprintConfig,
    {
        let runner = BlueprintRunner::new(config, env);
        TestRunner { inner: runner }
    }

    pub fn add_job<J>(&mut self, job: J) -> &mut Self
    where
        J: InitializableEventHandler + Send + Sync + 'static,
    {
        self.inner.job(job);
        self
    }

    pub fn add_background_service<B>(&mut self, service: B) -> &mut Self
    where
        B: BackgroundService + Send + 'static,
    {
        self.inner.background_service(Box::new(service));
        self
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        self.inner.run().await
    }
}

pub trait TestEnv: Sized {
    type Config: BlueprintConfig;

    fn new(config: Self::Config, env: GadgetConfiguration) -> Result<Self, Error>;
    fn add_job<J>(&mut self, job: J)
    where
        J: InitializableEventHandler + Send + Sync + 'static;
    fn add_background_service<B>(&mut self, service: B)
    where
        B: BackgroundService + Send + 'static;
    fn get_gadget_config(&self) -> GadgetConfiguration;
    fn run_runner(&self) -> impl std::future::Future<Output = Result<(), Error>> + Send;
}
