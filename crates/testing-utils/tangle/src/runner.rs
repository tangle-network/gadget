#![allow(dead_code)]

use blueprint_core::Job;
use blueprint_runner::BackgroundService;
use blueprint_runner::config::BlueprintEnvironment;
use blueprint_runner::config::Multiaddr;
use blueprint_runner::error::RunnerError as Error;
use blueprint_runner::tangle::config::TangleConfig;
use blueprint_tangle_extra::consumer::TangleConsumer;
use blueprint_tangle_extra::producer::TangleProducer;
use gadget_contexts::tangle::TangleClientContext;
use gadget_core_testing_utils::runner::{TestEnv, TestRunner};
use gadget_crypto_tangle_pair_signer::TanglePairSigner;
use gadget_keystore::backends::Backend;
use gadget_keystore::crypto::sp_core::SpSr25519;
use std::fmt::{Debug, Formatter};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

pub struct TangleTestEnv<Ctx> {
    pub runner: Option<TestRunner<Ctx>>,
    pub config: TangleConfig,
    pub env: BlueprintEnvironment,
    pub runner_handle: Mutex<Option<JoinHandle<Result<(), Error>>>>,
}

impl<Ctx> TangleTestEnv<Ctx> {
    pub(crate) fn update_networking_config(
        &mut self,
        bootnodes: Vec<Multiaddr>,
        network_bind_port: u16,
    ) {
        self.env.bootnodes = bootnodes;
        self.env.network_bind_port = network_bind_port;
    }

    // TODO(serial): This needs to return errors. Too many chances to panic here. Not helpful.
    pub(crate) async fn set_tangle_producer_consumer(&mut self) {
        let runner = self.runner.as_mut().expect("Runner already running");
        let builder = runner.builder.take().expect("Runner already running");
        let tangle_client = self
            .env
            .tangle_client()
            .await
            .expect("Tangle node should be running");
        let producer = TangleProducer::finalized_blocks(tangle_client.rpc_client.clone())
            .await
            .expect("Failed to create producer");

        let sr25519_signer = self
            .env
            .keystore()
            .first_local::<SpSr25519>()
            .expect("key not found");
        let sr25519_pair = self
            .env
            .keystore()
            .get_secret::<SpSr25519>(&sr25519_signer)
            .expect("key not found");
        let sr25519_signer = TanglePairSigner::new(sr25519_pair.0);
        let consumer = TangleConsumer::new(tangle_client.rpc_client.clone(), sr25519_signer);
        runner.builder = Some(builder.producer(producer).consumer(consumer));
    }
}

impl<Ctx> Debug for TangleTestEnv<Ctx> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TangleTestEnv")
            .field("config", &self.config)
            .field("env", &self.env)
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
            runner: Some(runner),
            config,
            env: env,
            runner_handle: Mutex::new(None),
        })
    }

    fn add_job<J, T>(&mut self, job: J)
    where
        J: Job<T, ()> + Send + Sync + 'static,
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
