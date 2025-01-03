use gadget_config::GadgetConfiguration;
use gadget_core_testing_utils::runner::{GenericTestEnv, RunnerSetup};
use gadget_runners::core::error::RunnerError as Error;
use gadget_runners::tangle::tangle::TangleConfig;

pub struct TangleTestEnv {
    generic_env: GenericTestEnv,
}

impl TangleTestEnv {
    pub fn new() -> Result<Self, Error> {
        let config = GadgetConfiguration::default();
        Ok(Self {
            generic_env: GenericTestEnv::new(config),
        })
    }

    pub async fn run_runner(&self, setup: RunnerSetup<TangleConfig>) -> Result<(), Error> {
        self.generic_env.run_runner(setup).await
    }

    pub async fn run_multiple_runners(
        &self,
        setups: Vec<RunnerSetup<TangleConfig>>,
    ) -> Result<(), Error> {
        self.generic_env.run_multiple_runners(setups).await
    }
}
