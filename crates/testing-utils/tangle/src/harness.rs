use crate::Error;
use crate::{
    keys::inject_tangle_key,
    node::{
        run,
        transactions::{self, setup_operator_and_service, submit_and_verify_job},
        NodeConfig,
    },
    runner::TangleTestEnv,
    InputValue, OutputValue,
};
use gadget_client_tangle::client::TangleClient;
use gadget_config::{supported_chains::SupportedChains, ContextConfig, GadgetConfiguration};
use gadget_contexts::{keystore::KeystoreContext, tangle::TangleClientContext};
use gadget_core_testing_utils::{
    harness::{BaseTestHarness, TestHarness},
    runner::TestEnv,
};
use gadget_crypto_tangle_pair_signer::TanglePairSigner;
use gadget_event_listeners::core::InitializableEventHandler;
use gadget_keystore::backends::Backend;
use gadget_keystore::crypto::sp_core::{SpEcdsa, SpSr25519};
use gadget_macros::ext::futures::stream::FuturesUnordered;
use gadget_macros::ext::futures::TryStreamExt;
use gadget_runners::core::error::RunnerError;
use gadget_logging::debug;
use gadget_runners::tangle::tangle::{PriceTargets, TangleConfig};
use sp_core::Pair;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use tangle_subxt::tangle_testnet_runtime::api::services::{
    calls::types::{call::Job, register::Preferences},
    events::JobResultSubmitted,
};
use tempfile::TempDir;
use tokio::sync::Mutex;
use tokio::task::JoinError;
use url::Url;

/// Configuration for the Tangle test harness
#[derive(Default)]
pub struct TangleTestConfig {
    pub http_endpoint: Option<Url>,
    pub ws_endpoint: Option<Url>,
}

/// Test harness for Tangle network tests
pub struct TangleTestHarness<const N: usize> {
    pub http_endpoint: Url,
    pub ws_endpoint: Url,
    client: TangleClient,
    pub sr25519_signer: TanglePairSigner<sp_core::sr25519::Pair>,
    pub ecdsa_signer: TanglePairSigner<sp_core::ecdsa::Pair>,
    pub alloy_key: alloy_signer_local::PrivateKeySigner,
    pub client_envs: Vec<GadgetConfiguration>,
    _temp_dir: tempfile::TempDir,
    _node: crate::node::testnet::SubstrateNode,
}

/// Manages multiple Tangle test environments and executes jobs in parallel across different configurations.
///
/// [`MultiNodeTangleTestEnv`] allows adding, starting, and stopping jobs on demand, facilitating concurrent testing
/// across multiple Tangle nodes. It handles the coordination of job execution and allows introspection on the state
/// of the system as a whole (either all jobs are running successfully, or a specific job has failed which it interpreted
/// as a global failure in the testing context.
pub struct MultiNodeTangleTestEnv {
    envs: Vec<GadgetConfiguration>,
    start_tx: Option<tokio::sync::oneshot::Sender<()>>,
    command_tx: tokio::sync::mpsc::UnboundedSender<NodeExecutorCommand>,
    dead_rx: Option<tokio::sync::oneshot::Receiver<RunnerError>>,
}

enum NodeExecutorCommand {
    StartJob(usize, Box<dyn JobCreator>),
    Stop(usize),
}

trait JobCreator: Fn(GadgetConfiguration) -> Box<dyn InitializableEventHandler> {}
impl<T: Fn(GadgetConfiguration) -> Box<dyn InitializableEventHandler>> JobCreator for T {}

impl MultiNodeTangleTestEnv {
    /// Creates a new `MultiNodeTangleTestEnv` that can execute jobs in parallel across multiple chains.
    ///
    /// The `test_envs` parameter should be a `HashMap` where the key is the job ID and the value is the
    /// `TangleTestEnv` to use for that job. The `envs` parameter should be a vector of
    /// `GadgetConfiguration`s that will be used to create the event handlers for each job.
    ///
    /// The `MultiNodeTangleTestEnv` will not execute any jobs until the `execute` method is called.
    /// When `execute` is called, the `MultiNodeTangleTestEnv` will start a background task that will
    /// execute the jobs in parallel. The background task will not shut down until all jobs have
    /// completed or the `MultiNodeTangleTestEnv` is dropped.
    ///
    /// The `MultiNodeTangleTestEnv` provides methods for starting and stopping jobs. The `start_job`
    /// method takes a job ID and a closure that returns an `InitializableEventHandler`. The closure
    /// will be called with the `GadgetConfiguration` for the specified job ID, and the returned event
    /// handler will be used to execute the job. The `stop_job` method takes a job ID and will signal
    /// the background task to stop the job with the specified ID.
    ///
    /// The `MultiNodeTangleTestEnv` also provides a method for getting the `GadgetConfiguration` for
    /// a given job ID. This can be used to get the configuration for a job without starting the job.
    pub async fn new(
        mut test_envs: HashMap<usize, TangleTestEnv>,
        envs: Vec<GadgetConfiguration>,
    ) -> Self {
        let (start_tx, start_rx) = tokio::sync::oneshot::channel();
        let (command_tx, mut command_rx) = tokio::sync::mpsc::unbounded_channel();
        let (dead_tx, dead_rx) = tokio::sync::oneshot::channel();
        let envs_clone = envs.clone();

        // This task will not run until the user has triggered it to begin
        let background_task = async move {
            if start_rx.await.is_err() {
                gadget_logging::warn!("MultiNodeTangleTestEnv was dropped without executing");
            }

            // Allows stopping a running job
            let mut handles = HashMap::new();

            while let Some(command) = command_rx.recv().await {
                match command {
                    NodeExecutorCommand::StartJob(job_id, creator) => {
                        let job = creator(envs_clone[job_id].clone());
                        if let Some(stopper_rx) = job.init_event_handler().await {
                            let (control_tx, mut control_rx) =
                                tokio::sync::oneshot::channel::<()>();
                            let task = async move {
                                tokio::select! {
                                    _ = control_rx => {
                                        gadget_logging::info!("[SHUTDOWN] Job {job_id} manually shutdown");
                                    },
                                    res = stopper_rx.recv() => {
                                        gadget_logging::info!("[SHUTDOWN] Job {job_id} completed with result {res}");
                                    }
                                };
                            };

                            handles.insert(job_id, control_tx);
                        } else {
                            gadget_logging::warn!(
                                "Failed to initialize event handler for job {job_id}"
                            );
                        }
                    }

                    NodeExecutorCommand::Stop(job_id) => {
                        gadget_logging::info!("[SHUTDOWN] Job {job_id} shutdown signal received");
                        if let Some(control_tx) = handles.remove(job_id) {
                            let _ = control_tx.send(());
                        } else {
                            gadget_logging::warn!("Failed to stop job {job_id}");
                        }
                    }
                }
            }
        };

        drop(tokio::spawn(background_task));

        Self {
            envs,
            command_tx,
            start_tx: Some(start_tx),
            dead_rx: Some(dead_rx),
        }
    }

    /// Adds a job to the test harness to be executed when the test is run.
    ///
    /// The `job_creator` parameter is a function that takes a `GadgetConfiguration` as an argument
    /// and returns a boxed `InitializableEventHandler`. The job creator is called with the
    /// `GadgetConfiguration` corresponding to the environment the job is running in.
    ///
    /// The job is added to the end of the list of jobs and can be stopped using the `stop_job`
    /// method.
    ///
    /// # Errors
    ///
    /// If the job cannot be added to the test harness, an error is returned.
    pub fn add_job<T: Fn(GadgetConfiguration) -> K, K: InitializableEventHandler>(
        &mut self,
        job_creator: T,
    ) -> Result<&mut Self, RunnerError> {
        self.command_tx
            .send(NodeExecutorCommand::StartJob(
                self.envs.len(),
                Box::new(job_creator),
            ))
            .map_err(|err| RunnerError::Other(err.to_string()))?;
        Ok(self)
    }

    /// Stops a job with the given job ID. Returns an error if the job is not found or if
    /// the job is not running.
    pub fn stop_job(&mut self, job_id: usize) -> Result<&mut Self, RunnerError> {
        self.command_tx
            .send(NodeExecutorCommand::Stop(job_id))
            .map_err(|err| RunnerError::Other(err.to_string()))?;
        Ok(self)
    }

    /// Begins the execution of the jobs in parallel and in the background
    ///
    /// Any jobs preloaded via `add_job` will be executed after this function is called
    /// Consequent jobs may still be added via `add_job`
    pub fn start(&mut self) -> Result<(), RunnerError> {
        self.start_tx
            .take()
            .ok_or_else(|| RunnerError::Other("Test harness already started".to_string()))?;
        /*
        Old logic for reference:
        let mut handles = FuturesUnordered::new();
        for test_env in &mut self.test_envs {
            handles.push(tokio::spawn(test_env.run_runner()));
        }

        async move {
            let mut results = handles.try_collect::<Vec<_>>().await
                .map_err(|err| RunnerError::Other(err.to_string()))?;
            results.pop().expect("Should have at least one result")
        }*/
        Ok(())
    }

    /// Returns `true` if the test harness has errored, `false` otherwise.
    ///
    /// If the test harness has errored, the `dead_rx` oneshot receiver is taken and
    /// awaited. If the receiver is `None`, `true` is returned immediately.
    ///
    /// The return value of this function is `true` if the test harness has errored,
    /// regardless of whether the `dead_rx` was taken and awaited or not.
    pub async fn has_errored(&mut self) -> bool {
        if let Some(mut rx) = self.dead_rx.take() {
            let _ = rx.await;
        }

        true
    }
}

const ENDOWED_TEST_NAMES: [&str; 5] = ["Alice", "Bob", "Charlie", "Dave", "Eve"];

#[async_trait::async_trait]
impl<const N: usize> TestHarness for TangleTestHarness<N> {
    type Config = TangleTestConfig;
    type Error = Error;

    async fn setup(test_dir: TempDir) -> Result<Self, Self::Error> {
        assert!(N <= 5, "Cannot setup more than 5 services");
        assert_ne!(N, 0, "Cannot setup 0 services");

        // Start Local Tangle Node
        let node = run(NodeConfig::new(false))
            .await
            .map_err(|e| Error::Setup(e.to_string()))?;
        let http_endpoint = Url::parse(&format!("http://127.0.0.1:{}", node.ws_port()))?;
        let ws_endpoint = Url::parse(&format!("ws://127.0.0.1:{}", node.ws_port()))?;

        let mut client_envs = vec![];

        for idx in 0..N {
            // Setup testing directory
            let name = ENDOWED_TEST_NAMES[idx];
            let test_dir_path = test_dir
                .path()
                .join(format!("./{}", name.to_ascii_lowercase()))
                .to_string_lossy()
                .into_owned();
            tokio::fs::create_dir_all(&test_dir_path).await?;
            inject_tangle_key(&test_dir_path, &format!("//{name}"))?;

            // Create context config
            let context_config = ContextConfig::create_tangle_config(
                http_endpoint.clone(),
                ws_endpoint.clone(),
                test_dir_path,
                None,
                SupportedChains::LocalTestnet,
                0,
                Some(0),
            );

            // Load environment
            let mut env =
                gadget_config::load(context_config).map_err(|e| Error::Setup(e.to_string()))?;

            // Always set test mode, dont require callers to set env vars
            env.test_mode = true;
            client_envs.push(env);
        }

        let alice_env = &client_envs[0];

        // Create config
        let config = TangleTestConfig {
            http_endpoint: Some(http_endpoint.clone()),
            ws_endpoint: Some(ws_endpoint.clone()),
        };

        // Setup signers
        let keystore = alice_env.keystore();
        let sr25519_public = keystore.first_local::<SpSr25519>()?;
        let sr25519_pair = keystore.get_secret::<SpSr25519>(&sr25519_public)?;
        let sr25519_signer = TanglePairSigner::new(sr25519_pair.0);

        let ecdsa_public = keystore.first_local::<SpEcdsa>()?;
        let ecdsa_pair = keystore.get_secret::<SpEcdsa>(&ecdsa_public)?;
        let ecdsa_signer = TanglePairSigner::new(ecdsa_pair.0);
        let alloy_key = ecdsa_signer
            .alloy_key()
            .map_err(|e| Error::Setup(e.to_string()))?;

        let client = alice_env.tangle_client().await?;
        let harness = Self {
            client_envs,
            http_endpoint,
            ws_endpoint,
            client,
            sr25519_signer,
            ecdsa_signer,
            alloy_key,
            _temp_dir: test_dir,
            _node: node,
        };

        // Deploy MBSM if needed
        harness
            .deploy_mbsm_if_needed()
            .await
            .map_err(|_| Error::Setup("Failed to deploy MBSM".to_string()))?;

        Ok(harness)
    }

    fn env(&self) -> &GadgetConfiguration {
        &self.client_envs[0]
    }
}

impl<const N: usize> TangleTestHarness<N> {
    /// Gets a reference to the Tangle client
    pub fn client(&self) -> &TangleClient {
        &self.client
    }

    /// Deploys MBSM if not already deployed
    async fn deploy_mbsm_if_needed(&self) -> Result<(), Error> {
        let latest_revision = transactions::get_latest_mbsm_revision(&self.client)
            .await
            .map_err(|e| Error::Setup(e.to_string()))?;

        if let Some((rev, addr)) = latest_revision {
            debug!("MBSM is deployed at revision #{rev} at address {addr}");
            return Ok(());
        } else {
            debug!("MBSM is not deployed");
        }

        let bytecode = tnt_core_bytecode::bytecode::MASTER_BLUEPRINT_SERVICE_MANAGER;
        transactions::deploy_new_mbsm_revision(
            self.ws_endpoint.as_str(),
            &self.client,
            &self.sr25519_signer,
            self.alloy_key.clone(),
            bytecode,
            alloy_primitives::address!("0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"), // TODO: User-defined address?
        )
        .await
        .map_err(|e| Error::Setup(e.to_string()))?;

        Ok(())
    }

    /// Creates deploy options for a blueprint
    pub fn create_deploy_opts(
        &self,
        manifest_path: std::path::PathBuf,
    ) -> cargo_tangle::deploy::tangle::Opts {
        cargo_tangle::deploy::tangle::Opts {
            pkg_name: Some(self.get_blueprint_name(&manifest_path)),
            http_rpc_url: self.http_endpoint.to_string(),
            ws_rpc_url: self.ws_endpoint.to_string(),
            manifest_path,
            signer: Some(self.sr25519_signer.clone()),
            signer_evm: Some(self.alloy_key.clone()),
        }
    }

    pub fn get_blueprint_name(&self, manifest_path: &std::path::Path) -> String {
        let manifest = gadget_core_testing_utils::read_cargo_toml_file(manifest_path)
            .expect("Failed to read blueprint's Cargo.toml");
        manifest.package.unwrap().name
    }

    pub fn get_default_operator_preferences(&self) -> Preferences {
        Preferences {
            key: gadget_runners::tangle::tangle::decompress_pubkey(
                &self.ecdsa_signer.signer().public().0,
            )
            .unwrap(),
            price_targets: PriceTargets::default().0,
        }
    }

    /// Deploys a blueprint from the current directory and returns its ID
    pub async fn deploy_blueprint(&self) -> Result<u64, Error> {
        let manifest_path = std::env::current_dir()?.join("Cargo.toml");
        let opts = self.create_deploy_opts(manifest_path);
        let blueprint_id = cargo_tangle::deploy::tangle::deploy_to_tangle(opts)
            .await
            .map_err(|e| Error::Setup(e.to_string()))?;
        Ok(blueprint_id)
    }

    /// Sets up a complete service environment with initialized event handlers
    ///
    /// # Returns
    /// A tuple of the test environment, the service ID, and the blueprint ID i.e., (test_env, service_id, blueprint_id)
    ///
    /// # Note
    /// The Service ID will always be 0 if automatic registration is disabled, as there is not yet a service to have an ID
    pub async fn setup_services(
        &self,
        exit_after_registration: bool,
    ) -> Result<(Vec<TangleTestEnv>, u64, u64), Error> {
        // Deploy blueprint
        let blueprint_id = self.deploy_blueprint().await?;

        // Join operators
        join_operators(&self.client, &self.sr25519_signer)
            .await
            .map_err(|e| Error::Setup(e.to_string()))?;

        // Setup operator and get service
        let preferences = self.get_default_operator_preferences();
        let service_id = if !exit_after_registration {
            setup_operator_and_service(
                &self.client,
                &self.sr25519_signer,
                blueprint_id,
                preferences,
                !exit_after_registration,
            )
            .await
            .map_err(|e| Error::Setup(e.to_string()))?
        } else {
            0
        };

        let config = TangleConfig::new(PriceTargets::default())
            .with_exit_after_register(exit_after_registration);

        // Create and spawn test environment
        let mut test_envs = vec![];
        for env in &self.client_envs {
            // Create and spawn test environment
            let test_env = TangleTestEnv::new(config.clone(), env.clone())?;
            test_envs.push(test_env);
        }

        Ok((test_envs, service_id, blueprint_id))
    }

    /// Requests a service with the given blueprint and returns the newly created service ID
    ///
    /// This function does not register for a service, it only requests service for a blueprint
    /// that has already been registered to.
    pub async fn request_service(&self, blueprint_id: u64) -> Result<u64, Error> {
        let preferences = self.get_default_operator_preferences();
        let service_id = setup_operator_and_service(
            &self.client,
            &self.sr25519_signer,
            blueprint_id,
            preferences,
            false,
        )
        .await
        .map_err(|e| Error::Setup(e.to_string()))?;

        Ok(service_id)
    }

    /// Executes a job and verifies its output matches the expected result
    ///
    /// # Arguments
    /// * `service_id` - The ID of the service to execute the job on
    /// * `job_id` - The ID of the job to execute
    /// * `inputs` - The input values for the job
    /// * `expected` - The expected output values
    ///
    /// # Returns
    /// The job results if execution was successful and outputs match expectations
    pub async fn execute_job(
        &self,
        service_id: u64,
        job_id: u8,
        inputs: Vec<InputValue>,
        expected: Vec<OutputValue>,
    ) -> Result<JobResultSubmitted, Error> {
        let results = submit_and_verify_job(
            &self.client,
            &self.sr25519_signer,
            service_id,
            Job::from(job_id),
            inputs,
            expected,
        )
        .await
        .map_err(|e| Error::Setup(e.to_string()))?;

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gadget_core::test_utils::assert_ok;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_harness_setup() {
        let test_dir = TempDir::new().unwrap();
        let harness = TangleTestHarness::<1>::setup(test_dir).await;
        assert!(harness.is_ok(), "Harness setup should succeed");

        let harness = harness.unwrap();
        assert!(harness.client().is_connected().await, "Client should be connected");
        assert_eq!(harness.client_envs.len(), 1, "Should have 1 client env");
    }

    #[tokio::test]
    async fn test_harness_setup_with_multiple_services() {
        let test_dir = TempDir::new().unwrap();
        let harness = TangleTestHarness::<3>::setup(test_dir).await;
        assert!(harness.is_ok(), "Harness setup should succeed");

        let harness = harness.unwrap();
        assert_eq!(harness.client_envs.len(), 3, "Should have 3 client envs");

        // Verify each environment has unique keys
        let keys: Vec<_> = harness
            .client_envs
            .iter()
            .map(|env| env.keystore().first_local::<SpSr25519>().unwrap())
            .collect();
        assert_eq!(keys.len(), 3, "Should have 3 unique keys");
        assert!(keys[0] != keys[1] && keys[1] != keys[2], "Keys should be unique");
    }

    #[tokio::test]
    async fn test_deploy_mbsm() {
        let test_dir = TempDir::new().unwrap();
        let harness = TangleTestHarness::<1>::setup(test_dir).await.unwrap();

        // MBSM should be deployed during setup
        let latest_revision = transactions::get_latest_mbsm_revision(harness.client())
            .await
            .unwrap();
        assert!(latest_revision.is_some(), "MBSM should be deployed");
    }

    #[tokio::test]
    async fn test_execute_job() {
        let test_dir = TempDir::new().unwrap();
        let harness = TangleTestHarness::<1>::setup(test_dir).await.unwrap();

        // First set up a service
        let (test_envs, service_id) = harness.setup_services().await.unwrap();
        assert!(!test_envs.is_empty(), "Should have test environments");
        assert!(service_id > 0, "Should have valid service ID");

        // Execute a simple job
        let inputs = vec![InputValue::U64(42)];
        let expected = vec![OutputValue::U64(42)];
        let result = harness.execute_job(service_id, 0, inputs, expected).await;
        assert_ok!(result, "Job execution should succeed");
    }

    #[tokio::test]
    async fn test_create_deploy_opts() {
        let test_dir = TempDir::new().unwrap();
        let harness = TangleTestHarness::<1>::setup(test_dir).await.unwrap();

        let manifest_path = PathBuf::from("Cargo.toml");
        let opts = harness.create_deploy_opts(manifest_path.clone());

        assert_eq!(opts.manifest_path, manifest_path);
        assert!(opts.signer.is_some(), "Should have SR25519 signer");
        assert!(opts.signer_evm.is_some(), "Should have EVM signer");
        assert!(opts.http_rpc_url.starts_with("http://"), "Should have HTTP URL");
        assert!(opts.ws_rpc_url.starts_with("ws://"), "Should have WebSocket URL");
    }

    #[tokio::test]
    #[should_panic(expected = "Cannot setup more than 5 services")]
    async fn test_harness_setup_exceeds_max_services() {
        let test_dir = TempDir::new().unwrap();
        let _harness = TangleTestHarness::<6>::setup(test_dir).await.unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "Cannot setup 0 services")]
    async fn test_harness_setup_zero_services() {
        let test_dir = TempDir::new().unwrap();
        let _harness = TangleTestHarness::<0>::setup(test_dir).await.unwrap();
    }
}
