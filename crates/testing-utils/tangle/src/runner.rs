#![allow(dead_code)]

use blueprint_core::{Job, JobCall};
use blueprint_runner::BackgroundService;
use blueprint_runner::config::BlueprintEnvironment;
use blueprint_runner::error::RunnerError as Error;
use blueprint_runner::tangle::config::TangleConfig;
use gadget_config::Multiaddr;
use gadget_core_testing_utils::runner::{TestEnv, TestRunner};
use std::fmt::{Debug, Formatter};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

pub struct TangleTestEnv<Ctx> {
    pub runner: TestRunner<Ctx>,
    pub config: TangleConfig,
    pub gadget_config: BlueprintEnvironment,
    pub runner_handle: Mutex<Option<JoinHandle<Result<(), Error>>>>,
}

impl<Ctx> TangleTestEnv<Ctx> {
    pub(crate) fn update_networking_config(
        &mut self,
        bootnodes: Vec<Multiaddr>,
        network_bind_port: u16,
    ) {
        self.gadget_config.bootnodes = bootnodes;
        self.gadget_config.network_bind_port = network_bind_port;
    }
}

impl<Ctx> Debug for TangleTestEnv<Ctx> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TangleTestEnv")
            .field("config", &self.config)
            .field("gadget_config", &self.gadget_config)
            .finish()
    }
}

impl<Ctx> TestEnv for TangleTestEnv<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
{
    type Config = TangleConfig;
    type Context = Ctx;

    fn new(config: Self::Config, env: BlueprintEnvironment, context: Ctx) -> Result<Self, Error> {
        let runner = TestRunner::new::<Self::Config>(config.clone(), env.clone(), context);

        Ok(Self {
            runner,
            config,
            gadget_config: env,
            runner_handle: Mutex::new(None),
        })
    }

    fn add_job<J>(&mut self, job: J)
    where
        J: Job<JobCall, ()> + Send + Sync + 'static,
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
            return Err(Error::Tangle("Failed to spawn runner task".to_string()));
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
                Err(Error::Tangle(format!(
                    "Runner failed during startup: {}",
                    e
                )))
            }
            Err(e) => {
                gadget_logging::error!("Runner task panicked: {}", e);
                Err(Error::Tangle(format!("Runner task panicked: {}", e)))
            }
        }
    }
}

impl<Ctx> Drop for TangleTestEnv<Ctx> {
    fn drop(&mut self) {
        futures::executor::block_on(async {
            let mut _guard = self.runner_handle.lock().await;
            if let Some(handle) = _guard.take() {
                handle.abort();
            }
        })
    }
}
