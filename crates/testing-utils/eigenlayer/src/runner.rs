#![allow(dead_code)]

use blueprint_runner::config::BlueprintEnvironment;
use gadget_core_testing_utils::runner::{TestEnv, TestRunner};
use gadget_event_listeners::core::InitializableEventHandler;
use gadget_macros::ext::futures;
use gadget_runners::core::error::RunnerError as Error;
use gadget_runners::core::runner::BackgroundService;
use gadget_runners::eigenlayer::bls::EigenlayerBLSConfig;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

pub struct EigenlayerBLSTestEnv {
    pub runner: TestRunner,
    pub config: EigenlayerBLSConfig,
    pub gadget_config: BlueprintEnvironment,
    pub runner_handle: Mutex<Option<JoinHandle<Result<(), Error>>>>,
}

impl TestEnv for EigenlayerBLSTestEnv {
    type Config = EigenlayerBLSConfig;

    fn new(config: Self::Config, env: BlueprintEnvironment) -> Result<Self, Error> {
        let runner = TestRunner::new(config, env.clone());

        Ok(Self {
            runner,
            config,
            gadget_config: env,
            runner_handle: Mutex::new(None),
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

    fn get_gadget_config(&self) -> BlueprintEnvironment {
        self.gadget_config.clone()
    }

    async fn run_runner(&self) -> Result<(), Error> {
        // Spawn the runner in a background task
        let mut runner = self.runner.clone();
        let handle = tokio::spawn(async move { runner.run().await });

        let mut _guard = self.runner_handle.lock().await;
        *_guard = Some(handle);

        // Brief delay to allow for startup
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Just check if it failed immediately
        let Some(handle) = _guard.take() else {
            return Err(Error::Eigenlayer("Failed to spawn runner task".to_string()));
        };

        if !handle.is_finished() {
            // Put the handle back since the runner is still running
            *_guard = Some(handle);
            gadget_logging::info!("Runner started successfully");
            return Ok(());
        }

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
    }
}

impl Drop for EigenlayerBLSTestEnv {
    fn drop(&mut self) {
        futures::executor::block_on(async {
            let mut _guard = self.runner_handle.lock().await;
            if let Some(handle) = _guard.take() {
                handle.abort();
            }
        })
    }
}
