use crate::node::transactions::join_operators;
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
use gadget_keystore::backends::Backend;
use gadget_keystore::crypto::sp_core::{SpEcdsa, SpSr25519};
use gadget_logging::debug;
use gadget_runners::tangle::tangle::{PriceTargets, TangleConfig};
use sp_core::Pair;
use tangle_subxt::tangle_testnet_runtime::api::services::{
    calls::types::{call::Job, register::Preferences},
    events::JobResultSubmitted,
};
use tempfile::TempDir;
use url::Url;

/// Configuration for the Tangle test harness
#[derive(Default)]
pub struct TangleTestConfig {
    pub http_endpoint: Option<Url>,
    pub ws_endpoint: Option<Url>,
}

/// Test harness for Tangle network tests
pub struct TangleTestHarness {
    base: BaseTestHarness<TangleTestConfig>,
    pub http_endpoint: Url,
    pub ws_endpoint: Url,
    client: TangleClient,
    pub sr25519_signer: TanglePairSigner<sp_core::sr25519::Pair>,
    pub ecdsa_signer: TanglePairSigner<sp_core::ecdsa::Pair>,
    pub alloy_key: alloy_signer_local::PrivateKeySigner,
    _temp_dir: tempfile::TempDir,
    _node: crate::node::testnet::SubstrateNode,
}

#[async_trait::async_trait]
impl TestHarness for TangleTestHarness {
    type Config = TangleTestConfig;
    type Error = Error;

    async fn setup(test_dir: TempDir) -> Result<Self, Self::Error> {
        // Start Local Tangle Node
        let node = run(NodeConfig::new(false))
            .await
            .map_err(|e| Error::Setup(e.to_string()))?;
        let http_endpoint = Url::parse(&format!("http://127.0.0.1:{}", node.ws_port()))?;
        let ws_endpoint = Url::parse(&format!("ws://127.0.0.1:{}", node.ws_port()))?;

        // Setup testing directory
        let test_dir_path = test_dir.path().to_string_lossy().into_owned();
        inject_tangle_key(&test_dir_path, "//Alice")?;

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

        // Create config
        let config = TangleTestConfig {
            http_endpoint: Some(http_endpoint.clone()),
            ws_endpoint: Some(ws_endpoint.clone()),
        };

        let base = BaseTestHarness::new(env.clone(), config);

        // Setup signers
        let keystore = env.keystore();
        let sr25519_public = keystore.first_local::<SpSr25519>()?;
        let sr25519_pair = keystore.get_secret::<SpSr25519>(&sr25519_public)?;
        let sr25519_signer = TanglePairSigner::new(sr25519_pair.0);

        let ecdsa_public = keystore.first_local::<SpEcdsa>()?;
        let ecdsa_pair = keystore.get_secret::<SpEcdsa>(&ecdsa_public)?;
        let ecdsa_signer = TanglePairSigner::new(ecdsa_pair.0);
        let alloy_key = ecdsa_signer
            .alloy_key()
            .map_err(|e| Error::Setup(e.to_string()))?;

        let client = env.tangle_client().await?;
        let harness = Self {
            base,
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
        &self.base.env
    }

    fn config(&self) -> &Self::Config {
        &self.base.config
    }
}

impl TangleTestHarness {
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
        automatic_registration: bool,
    ) -> Result<(TangleTestEnv, u64, u64), Error> {
        // Deploy blueprint
        let blueprint_id = self.deploy_blueprint().await?;

        // Join operators
        join_operators(&self.client, &self.sr25519_signer)
            .await
            .map_err(|e| Error::Setup(e.to_string()))?;

        // Setup operator and get service
        let preferences = self.get_default_operator_preferences();
        let service_id = if automatic_registration {
            setup_operator_and_service(
                &self.client,
                &self.sr25519_signer,
                blueprint_id,
                preferences,
                automatic_registration,
            )
            .await
            .map_err(|e| Error::Setup(e.to_string()))?
        } else {
            0
        };

        let config =
            TangleConfig::new(PriceTargets::default()).with_pre_register(!automatic_registration);

        // Create and spawn test environment
        let test_env = TangleTestEnv::new(config, self.env().clone())?;

        Ok((test_env, service_id, blueprint_id))
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
