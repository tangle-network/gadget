#![allow(dead_code)]
use gadget_config::GadgetConfiguration;
use gadget_core_testing_utils::runner::{TestEnv, TestRunner};
use gadget_event_listeners::core::InitializableEventHandler;
use gadget_runners::core::error::RunnerError as Error;
use gadget_runners::core::runner::BackgroundService;
use gadget_runners::eigenlayer::bls::EigenlayerBLSConfig;

pub struct EigenlayerBLSTestEnv {
    runner: TestRunner,
    config: EigenlayerBLSConfig,
    gadget_config: GadgetConfiguration,
}

impl TestEnv for EigenlayerBLSTestEnv {
    type Config = EigenlayerBLSConfig;

    fn new(config: Self::Config, env: GadgetConfiguration) -> Result<Self, Error> {
        let runner = TestRunner::new(config, env.clone());

        Ok(Self {
            runner,
            config,
            gadget_config: env,
        })
    }

    fn add_job<J>(&mut self, job: J)
    where
        J: InitializableEventHandler + Send + 'static,
    {
        self.runner.add_job(job);
    }

    fn add_background_service<B>(&mut self, service: B)
    where
        B: BackgroundService + Send + 'static,
    {
        self.runner.add_background_service(service);
    }

    fn get_gadget_config(self) -> GadgetConfiguration {
        self.gadget_config.clone()
    }

    async fn run_runner(&mut self) -> Result<(), Error> {
        self.runner.run().await
    }
}
