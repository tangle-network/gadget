#![allow(dead_code)]

use blueprint_core::Job;
use blueprint_runner::BackgroundService;
use blueprint_runner::config::BlueprintEnvironment;
use blueprint_runner::eigenlayer::bls::EigenlayerBLSConfig;
use blueprint_runner::error::RunnerError as Error;
use gadget_core_testing_utils::runner::{TestEnv, TestRunner};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

pub struct EigenlayerBLSTestEnv<Ctx> {
    pub runner: Option<TestRunner<Ctx>>,
    pub config: EigenlayerBLSConfig,
    pub env: BlueprintEnvironment,
    pub runner_handle: Mutex<Option<JoinHandle<Result<(), Error>>>>,
}

impl<Ctx> TestEnv for EigenlayerBLSTestEnv<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
{
    type Config = EigenlayerBLSConfig;
    type Context = Ctx;

    fn new(
        config: Self::Config,
        env: BlueprintEnvironment,
        context: Self::Context,
    ) -> Result<Self, Error> {
        let runner = TestRunner::new(config, env.clone(), context);

        Ok(Self {
            runner: Some(runner),
            config,
            env,
            runner_handle: Mutex::new(None),
        })
    }

    fn add_job<J, T>(&mut self, job: J)
    where
        J: Job<T, Self::Context> + Send + Sync + 'static,
        T: 'static,
    {
        self.runner
            .as_mut()
            .expect("Runner already running")
            .add_job(job);
    }

    fn add_background_service<B>(&mut self, service: B)
    where
        B: BackgroundService + Send + 'static,
    {
        self.runner
            .as_mut()
            .expect("Runner already running")
            .add_background_service(service);
    }

    fn get_gadget_config(&self) -> BlueprintEnvironment {
        self.env.clone()
    }

    async fn run_runner(&mut self) -> Result<(), Error> {
        // Spawn the runner in a background task
        let runner = self.runner.take().expect("Runner already running");
        let handle = tokio::spawn(async move { runner.run().await });

        let mut guard = self.runner_handle.lock().await;
        *guard = Some(handle);

        // Brief delay to allow for startup
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Just check if it failed immediately
        let Some(handle) = guard.take() else {
            return Err(Error::Eigenlayer("Failed to spawn runner task".to_string()));
        };

        if !handle.is_finished() {
            // Put the handle back since the runner is still running
            *guard = Some(handle);
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

impl<Ctx> Drop for EigenlayerBLSTestEnv<Ctx> {
    fn drop(&mut self) {
        futures::executor::block_on(async {
            let mut guard = self.runner_handle.lock().await;
            if let Some(handle) = guard.take() {
                handle.abort();
            }
        });
    }
}
