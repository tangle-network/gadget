use crate::Error;
use crate::multi_node::MultiNodeTestEnv;
use crate::node::transactions::setup_operator_and_service_multiple;
use crate::{
    InputValue, OutputValue,
    keys::inject_tangle_key,
    node::{NodeConfig, run, transactions},
};
use blueprint_core::debug;
use blueprint_runner::config::BlueprintEnvironment;
use blueprint_runner::config::ContextConfig;
use blueprint_runner::config::SupportedChains;
use blueprint_runner::error::RunnerError;
use blueprint_runner::tangle::config::PriceTargets;
use gadget_client_tangle::client::TangleClient;
use gadget_contexts::tangle::TangleClientContext;
use gadget_crypto_tangle_pair_signer::TanglePairSigner;
use gadget_keystore::backends::Backend;
use gadget_keystore::crypto::sp_core::{SpEcdsa, SpSr25519};
use std::io;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use tangle_subxt::tangle_testnet_runtime::api::services::events::JobCalled;
use tangle_subxt::tangle_testnet_runtime::api::services::{
    calls::types::{call::Job, register::Preferences},
    events::JobResultSubmitted,
};
use tempfile::TempDir;
use url::Url;

pub const ENDOWED_TEST_NAMES: [&str; 10] = [
    "Alice",
    "Bob",
    "Charlie",
    "Dave",
    "Eve",
    "Ferdinand",
    "Gina",
    "Hank",
    "Ivy",
    "Jack",
];

/// Configuration for the Tangle test harness
#[derive(Clone, Debug)]
pub struct TangleTestConfig {
    pub http_endpoint: Url,
    pub ws_endpoint: Url,
    pub temp_dir: PathBuf,
}

/// Test harness for Tangle network tests
pub struct TangleTestHarness<Ctx = ()> {
    pub http_endpoint: Url,
    pub ws_endpoint: Url,
    client: TangleClient,
    pub sr25519_signer: TanglePairSigner<sp_core::sr25519::Pair>,
    pub ecdsa_signer: TanglePairSigner<sp_core::ecdsa::Pair>,
    pub alloy_key: alloy_signer_local::PrivateKeySigner,
    config: TangleTestConfig,
    temp_dir: tempfile::TempDir,
    _node: crate::node::testnet::SubstrateNode,
    _phantom: PhantomData<Ctx>,
}

pub(crate) async fn generate_env_from_node_id(
    identity: &str,
    http_endpoint: Url,
    ws_endpoint: Url,
    test_dir: &Path,
) -> Result<BlueprintEnvironment, RunnerError> {
    let keystore_path = test_dir.join(identity.to_ascii_lowercase());
    tokio::fs::create_dir_all(&keystore_path).await?;
    inject_tangle_key(&keystore_path, &format!("//{identity}"))
        .map_err(|err| RunnerError::Other(err.to_string()))?;

    // Create context config
    let context_config = ContextConfig::create_tangle_config(
        http_endpoint,
        ws_endpoint,
        keystore_path.display().to_string(),
        None,
        SupportedChains::LocalTestnet,
        0,
        Some(0),
    );

    // Load environment
    let mut env = BlueprintEnvironment::load_with_config(context_config)
        .map_err(|e| Error::Setup(e.to_string()))
        .map_err(|err| RunnerError::Other(err.to_string()))?;

    // Always set test mode, dont require callers to set env vars
    env.test_mode = true;

    Ok(env)
}

impl<Ctx> TangleTestHarness<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
{
    /// Create a new `TangleTestHarness`
    ///
    /// NOTE: The resulting harness will have a context of `()`. This is not valid for jobs that require
    ///       a context. See [`Self::setup_with_context()`] and [`Self::set_context()`].
    ///
    /// This is useful for cases where:
    ///
    /// * None of the jobs require a context
    /// * The context creation depends on [`Self::env()`]
    ///
    /// # Errors
    ///
    /// * See [`Self::setup_with_context()`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use gadget_tangle_testing_utils::TangleTestHarness;
    /// use tempfile::TempDir;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let tmp_dir = TempDir::new()?;
    /// let harness = TangleTestHarness::setup(tmp_dir).await?;
    /// # Ok(()) }
    /// ```
    pub async fn setup(test_dir: TempDir) -> Result<Self, Error> {
        // Start Local Tangle Node
        let node = run(NodeConfig::new(false))
            .await
            .map_err(|e| Error::Setup(e.to_string()))?;
        let http_endpoint = Url::parse(&format!("http://127.0.0.1:{}", node.ws_port()))?;
        let ws_endpoint = Url::parse(&format!("ws://127.0.0.1:{}", node.ws_port()))?;

        // Alice idx = 0
        let alice_env = generate_env_from_node_id(
            ENDOWED_TEST_NAMES[0],
            http_endpoint.clone(),
            ws_endpoint.clone(),
            test_dir.path(),
        )
        .await?;

        // Create config
        let config = TangleTestConfig {
            http_endpoint: http_endpoint.clone(),
            ws_endpoint: ws_endpoint.clone(),
            temp_dir: test_dir.path().to_path_buf(),
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
        let harness = TangleTestHarness {
            http_endpoint,
            ws_endpoint,
            client,
            sr25519_signer,
            ecdsa_signer,
            alloy_key,
            temp_dir: test_dir,
            config,
            _node: node,
            _phantom: PhantomData,
        };

        // Deploy MBSM if needed
        harness
            .deploy_mbsm_if_needed()
            .await
            .map_err(|e| Error::Setup(format!("Failed to deploy MBSM: {e}")))?;

        Ok(harness)
    }

    #[must_use]
    pub fn env(&self) -> &BlueprintEnvironment {
        &self.client.config
    }
}

struct NodeInfo {
    env: BlueprintEnvironment,
    client: TangleClient,
    preferences: Preferences,
}

impl<Ctx> TangleTestHarness<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
{
    async fn get_all_node_info<const N: usize>(&self) -> Result<Vec<NodeInfo>, RunnerError> {
        let mut nodes = vec![];

        for name in &ENDOWED_TEST_NAMES[..N] {
            let env = generate_env_from_node_id(
                name,
                self.http_endpoint.clone(),
                self.ws_endpoint.clone(),
                self.temp_dir.path(),
            )
            .await?;

            let client = env
                .tangle_client()
                .await
                .map_err(|err| RunnerError::Other(err.to_string()))?;

            let keystore = env.keystore();
            let ecdsa_public = keystore
                .first_local::<SpEcdsa>()
                .map_err(|err| RunnerError::Other(err.to_string()))?;

            let preferences = Preferences {
                key: blueprint_runner::tangle::config::decompress_pubkey(&ecdsa_public.0.0)
                    .unwrap(),
                price_targets: PriceTargets::default().0,
            };

            nodes.push(NodeInfo {
                env,
                client,
                preferences,
            });
        }

        Ok(nodes)
    }

    /// Gets a reference to the Tangle client
    #[must_use]
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
        }

        debug!("MBSM is not deployed");

        let bytecode = tnt_core_bytecode::bytecode::MASTER_BLUEPRINT_SERVICE_MANAGER;
        transactions::deploy_new_mbsm_revision(
            self.ws_endpoint.as_str(),
            &self.client,
            &self.sr25519_signer,
            self.alloy_key.clone(),
            bytecode,
            alloy_primitives::address!("0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"), // TODO: User-defined address
        )
        .await
        .map_err(|e| Error::Setup(e.to_string()))?;

        Ok(())
    }

    /// Creates deploy options for a blueprint
    ///
    /// # Errors
    ///
    /// See [`read_cargo_toml_file()`]
    ///
    /// [`read_cargo_toml_file()`]: gadget_core_testing_utils::read_cargo_toml_file
    pub fn create_deploy_opts(
        &self,
        manifest_path: std::path::PathBuf,
    ) -> io::Result<cargo_tangle::deploy::tangle::Opts> {
        Ok(cargo_tangle::deploy::tangle::Opts {
            pkg_name: Some(self.get_blueprint_name(&manifest_path)?),
            http_rpc_url: self.http_endpoint.to_string(),
            ws_rpc_url: self.ws_endpoint.to_string(),
            manifest_path,
            signer: Some(self.sr25519_signer.clone()),
            signer_evm: Some(self.alloy_key.clone()),
        })
    }

    #[allow(clippy::unused_self)]
    fn get_blueprint_name(&self, manifest_path: &std::path::Path) -> io::Result<String> {
        let manifest = gadget_core_testing_utils::read_cargo_toml_file(manifest_path)?;
        Ok(manifest.package.unwrap().name)
    }

    /// Deploys a blueprint from the current directory and returns its ID
    ///
    /// # Errors
    ///
    /// See [`deploy_to_tangle()`]
    ///
    /// [`deploy_to_tangle()`]: cargo_tangle::deploy::tangle::deploy_to_tangle
    pub async fn deploy_blueprint(&self) -> Result<u64, Error> {
        let manifest_path = std::env::current_dir()?.join("Cargo.toml");
        let opts = self.create_deploy_opts(manifest_path)?;
        let blueprint_id = cargo_tangle::deploy::tangle::deploy_to_tangle(opts)
            .await
            .map_err(|e| Error::Setup(e.to_string()))?;
        Ok(blueprint_id)
    }

    /// Sets up a complete service environment with initialized event handlers
    ///
    /// # Returns
    /// A tuple of the test environment, the service ID, and the blueprint ID i.e., (`test_env`, `service_id`, `blueprint_id`)
    ///
    /// # Note
    /// The Service ID will always be 0 if automatic registration is disabled, as there is not yet a service to have an ID
    ///
    /// # Errors
    ///
    /// * See [`Self::deploy_blueprint()`] and [`MultiNodeTestEnv::new()`]
    pub async fn setup_services<const N: usize>(
        &self,
        exit_after_registration: bool,
    ) -> Result<(MultiNodeTestEnv<Ctx>, u64, u64), Error> {
        const { assert!(N > 0, "Must have at least 1 initial node") };

        // Deploy blueprint
        let blueprint_id = self.deploy_blueprint().await?;

        let nodes = self.get_all_node_info::<N>().await?;

        // Setup operator and get service
        let service_id = if exit_after_registration {
            0
        } else {
            let mut all_clients = Vec::new();
            let mut all_signers = Vec::new();
            let mut all_preferences = Vec::new();

            for node in nodes {
                let keystore = node.env.keystore();
                let sr25519_public = keystore
                    .first_local::<SpSr25519>()
                    .map_err(|err| RunnerError::Other(err.to_string()))?;
                let sr25519_pair = keystore
                    .get_secret::<SpSr25519>(&sr25519_public)
                    .map_err(|err| RunnerError::Other(err.to_string()))?;
                let sr25519_signer = TanglePairSigner::new(sr25519_pair.0);
                all_clients.push(node.client);
                all_signers.push(sr25519_signer);
                all_preferences.push(node.preferences);
            }

            setup_operator_and_service_multiple(
                &all_clients[..N],
                &all_signers[..N],
                blueprint_id,
                &all_preferences,
                exit_after_registration,
            )
            .await
            .map_err(|e| Error::Setup(e.to_string()))?
        };

        // Create and initialize the new multi-node environment
        let executor = MultiNodeTestEnv::new::<N>(self.config.clone());

        Ok((executor, service_id, blueprint_id))
    }

    /// Submits a job to be executed
    ///
    /// # Arguments
    /// * `service_id` - The ID of the service to submit the job to
    /// * `job_id` - The ID of the job to submit
    /// * `inputs` - The input values for the job
    ///
    /// # Returns
    /// The submitted job if successful
    ///
    /// # Errors
    ///
    /// Returns an error if the transaction fails
    pub async fn submit_job(
        &self,
        service_id: u64,
        job_id: u8,
        inputs: Vec<InputValue>,
    ) -> Result<JobCalled, Error> {
        let job = transactions::submit_job(
            &self.client,
            &self.sr25519_signer,
            service_id,
            Job::from(job_id),
            inputs,
            0, // TODO: Should this take a call ID? or leave it up to the caller to verify?
        )
        .await
        .map_err(|e| Error::Setup(e.to_string()))?;

        Ok(job)
    }

    /// Executes a previously submitted job and waits for completion
    ///
    /// # Arguments
    /// * `service_id` - The ID of the service the job was submitted to
    /// * `job` - The submitted job to execute
    ///
    /// # Returns
    /// The job results if execution was successful
    ///
    /// # Errors
    ///
    /// Returns an error if no job result is found.
    pub async fn wait_for_job_execution(
        &self,
        service_id: u64,
        job: JobCalled,
    ) -> Result<JobResultSubmitted, Error> {
        let results = transactions::wait_for_completion_of_tangle_job(
            &self.client,
            service_id,
            job.call_id,
            1,
        )
        .await
        .map_err(|e| Error::Setup(e.to_string()))?;

        Ok(results)
    }

    /// Verifies that job results match expected outputs
    ///
    /// # Arguments
    /// * `results` - The actual job results
    /// * `expected` - The expected output values
    ///
    /// # Returns
    /// The verified results if they match expectations
    ///
    /// # Panics
    ///
    /// If the results don't match the expected outputs
    pub fn verify_job(&self, results: &JobResultSubmitted, expected: impl AsRef<[OutputValue]>) {
        assert_eq!(
            results.result.len(),
            expected.as_ref().len(),
            "Number of outputs doesn't match expected"
        );

        for (result, expected) in results.result.iter().zip(expected.as_ref().iter()) {
            assert_eq!(result, expected);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_harness_setup() {
        let test_dir = TempDir::new().unwrap();
        let harness = TangleTestHarness::<()>::setup(test_dir).await;
        assert!(harness.is_ok(), "Harness setup should succeed");

        let harness = harness.unwrap();
        assert!(
            harness.client().now().await.is_some(),
            "Client should be connected to live chain"
        );
    }

    #[tokio::test]
    async fn test_deploy_mbsm() {
        let test_dir = TempDir::new().unwrap();
        let harness = TangleTestHarness::<()>::setup(test_dir).await.unwrap();

        // MBSM should be deployed during setup
        let latest_revision = transactions::get_latest_mbsm_revision(harness.client())
            .await
            .unwrap();
        assert!(latest_revision.is_some(), "MBSM should be deployed");
    }
}
