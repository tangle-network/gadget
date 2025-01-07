use gadget_config::GadgetConfiguration;
use gadget_runners::core::config::BlueprintConfig;
use gadget_runners::core::error::RunnerError as Error;
use gadget_runners::core::runner::BlueprintRunner;
use gadget_event_listeners::core::InitializableEventHandler;
use gadget_runners::core::jobs::JobBuilder;

pub struct TestRunner {
    inner: BlueprintRunner,
}

impl TestRunner {
    pub fn new<J, T, C>(config: C, env: GadgetConfiguration, jobs: Vec<J>) -> Self
    where
        J: Into<JobBuilder<T>> + 'static,
        T: InitializableEventHandler + Send + 'static,
        C: BlueprintConfig,
    {
        let mut runner = BlueprintRunner::new(config, env);

        for job in jobs.into_iter() {
            let job: JobBuilder<T> = job.into();
            runner.job(job);
        }

        TestRunner { inner: runner }
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        self.inner.run().await
    }
}

pub trait TestEnv {
    type Config: BlueprintConfig;

    fn new<J, T>(config: Self::Config, env: GadgetConfiguration, jobs: Vec<J>) -> Result<Self, Error>
    where
        J: Into<JobBuilder<T>> + 'static,
        T: InitializableEventHandler + Send + 'static,
        Self: Sized;
    fn get_gadget_config(self) -> GadgetConfiguration;
    fn run_runner(&mut self) -> impl std::future::Future<Output = Result<(), Error>> + Send;
}

pub struct GenericTestEnv<E: TestEnv> {
    env: E,
}

impl<E: TestEnv> GenericTestEnv<E> {
    pub fn new<J, EH>(config: E::Config, env: GadgetConfiguration, jobs: Vec<J>) -> Result<Self, Error>
    where
        J: Into<JobBuilder<EH>> + 'static,
        EH: InitializableEventHandler + Send + 'static,
    {
        Ok(Self { env: E::new(config, env, jobs)? })
    }

    pub async fn run_runner(&mut self) -> Result<(), Error> {
        self.env.run_runner().await
    }
}
