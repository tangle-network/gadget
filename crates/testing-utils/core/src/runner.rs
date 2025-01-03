use gadget_config::GadgetConfiguration;
use gadget_logging::info;
use gadget_runners::core::config::BlueprintConfig;
use gadget_runners::core::error::RunnerError as Error;
use gadget_runners::core::runner::BlueprintRunner;
use std::future::Future;
use std::pin::Pin;

type RunnerSetupFn = Box<
    dyn FnOnce(&mut BlueprintRunner) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send>>
        + Send,
>;

pub struct RunnerSetup<C: BlueprintConfig> {
    pub config: C,
    pub setup: RunnerSetupFn,
}

impl<C: BlueprintConfig> RunnerSetup<C> {
    pub fn new<F, Fut>(config: C, setup: F) -> Self
    where
        F: FnOnce(&mut BlueprintRunner) -> Fut + Send + 'static,
        Fut: Future<Output = Result<(), Error>> + Send + 'static,
    {
        RunnerSetup {
            config,
            setup: Box::new(move |runner| Box::pin(setup(runner))),
        }
    }
}

pub struct GenericTestEnv {
    pub config: GadgetConfiguration,
}

impl GenericTestEnv {
    pub fn new(config: GadgetConfiguration) -> Self {
        Self { config }
    }

    pub async fn run_runner<C: BlueprintConfig>(&self, setup: RunnerSetup<C>) -> Result<(), Error> {
        info!("Setting up runner test environment");
        let mut runner = BlueprintRunner::new(setup.config, self.config.clone());

        (setup.setup)(&mut runner).await
    }

    pub async fn run_multiple_runners<C: BlueprintConfig>(
        &self,
        setups: Vec<RunnerSetup<C>>,
    ) -> Result<(), Error> {
        for setup in setups {
            self.run_runner(setup).await?;
        }
        Ok(())
    }
}
