use gadget_config::GadgetConfiguration;
use gadget_core_testing_utils::runner::{TestEnv, TestRunner};
use gadget_runners::core::error::RunnerError as Error;
use gadget_runners::core::jobs::JobBuilder;
use gadget_runners::tangle::tangle::TangleConfig;
use gadget_event_listeners::core::InitializableEventHandler;

pub struct TangleTestEnv {
    runner: TestRunner,
    config: TangleConfig,
    gadget_config: GadgetConfiguration,
}

impl TestEnv for TangleTestEnv {
    type Config = TangleConfig;

    fn new<J, T>(config: Self::Config, env: GadgetConfiguration, jobs: Vec<J>) -> Result<Self, Error>
    where
        J: Into<JobBuilder<T>> + 'static,
        T: InitializableEventHandler + Send + 'static,
    {
        let gadget_config = GadgetConfiguration::default();
        let config = TangleConfig::default();
        let runner = TestRunner::new::<J, T, Self::Config>(config.clone(), gadget_config.clone(), vec![]);

        Ok(Self {
            runner,
            config,
            gadget_config,
        })
    }

    fn get_gadget_config(self) -> GadgetConfiguration {
        self.gadget_config.clone()
    }

    async fn run_runner(&mut self) -> Result<(), Error> {
        self.runner.run().await
    }
}
