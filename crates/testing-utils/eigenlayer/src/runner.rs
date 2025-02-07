#![allow(dead_code)]
use gadget_config::GadgetConfiguration;
use gadget_core_testing_utils::runner::{TestEnv, TestRunner};
use gadget_event_listeners::core::InitializableEventHandler;
use gadget_runners::core::error::RunnerError as Error;
use gadget_runners::core::runner::BackgroundService;
use gadget_runners::eigenlayer::bls::EigenlayerBLSConfig;
use tokio::task::JoinHandle;

pub struct EigenlayerBLSTestEnv {
    runner: TestRunner,
    config: EigenlayerBLSConfig,
    gadget_config: GadgetConfiguration,
    runner_handle: Option<JoinHandle<Result<(), Error>>>,
}

impl TestEnv for EigenlayerBLSTestEnv {
    type Config = EigenlayerBLSConfig;

    fn new(config: Self::Config, env: GadgetConfiguration) -> Result<Self, Error> {
        let runner = TestRunner::new(config, env.clone());

        Ok(Self {
            runner,
            config,
            gadget_config: env,
            runner_handle: None,
        })
    }

    fn add_job<J>(&mut self, job: J)
    where
        J: InitializableEventHandler + Send + Sync + 'static,
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
        // Spawn the runner in a background task
        let mut runner = self.runner.clone();
        let handle = tokio::spawn(async move { runner.run().await });

        self.runner_handle = Some(handle);

        // Brief delay to allow for startup
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Just check if it failed immediately
        if let Some(handle) = self.runner_handle.take() {
            if handle.is_finished() {
                gadget_logging::info!("Runner task finished OK");
                match handle.await {
                    Ok(Ok(())) => Ok(()),
                    Ok(Err(e)) => {
                        gadget_logging::error!("Runner failed during startup: {}", e);
                        Err(Error::Eigenlayer(format!(
                            "Runner failed during startup: {}",
                            e
                        )))
                    }
                    Err(e) => {
                        gadget_logging::error!("Runner task panicked: {}", e);
                        Err(Error::Eigenlayer(format!("Runner task panicked: {}", e)))
                    }
                }
            } else {
                // Put the handle back since the runner is still running
                self.runner_handle = Some(handle);
                gadget_logging::info!("Runner started successfully");
                Ok(())
            }
        } else {
            Err(Error::Eigenlayer("Failed to spawn runner task".to_string()))
        }
    }
}

impl Drop for EigenlayerBLSTestEnv {
    fn drop(&mut self) {
        if let Some(handle) = &self.runner_handle {
            handle.abort();
        }
    }
}
