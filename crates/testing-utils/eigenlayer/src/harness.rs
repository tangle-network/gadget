use crate::Error;
use crate::env::{EigenlayerTestEnvironment, setup_eigenlayer_test_environment};
use alloy_primitives::Address;
use alloy_provider::RootProvider;
use blueprint_evm_extra::util::get_provider_http;
use blueprint_runner::config::{BlueprintEnvironment, ContextConfig, SupportedChains};
use blueprint_runner::eigenlayer::config::EigenlayerProtocolSettings;
use gadget_chain_setup::anvil::keys::{ANVIL_PRIVATE_KEYS, inject_anvil_key};
use gadget_chain_setup::anvil::{Container, start_default_anvil_testnet};
use std::marker::PhantomData;
use tempfile::TempDir;
use url::Url;

/// Configuration for the Eigenlayer test harness
#[derive(Default)]
pub struct EigenlayerTestConfig {
    pub http_endpoint: Option<Url>,
    pub ws_endpoint: Option<Url>,
    pub eigenlayer_contract_addresses: Option<EigenlayerProtocolSettings>,
}

/// Test harness for Eigenlayer network tests
pub struct EigenlayerTestHarness<Ctx> {
    env: BlueprintEnvironment,
    config: EigenlayerTestConfig,
    pub http_endpoint: Url,
    pub ws_endpoint: Url,
    pub accounts: Vec<Address>,
    pub eigenlayer_contract_addresses: EigenlayerProtocolSettings,
    _temp_dir: TempDir,
    _container: Container,
    _phantom: PhantomData<Ctx>,
}

impl EigenlayerTestHarness<()> {
    /// Create a new `EigenlayerTestHarness`
    ///
    /// NOTE: The resulting harness will have a context of `()`. This is not valid for jobs that require
    ///       a context. See [`Self::setup_with_context()`] and [`Self::set_context()`].
    ///
    /// # Errors
    ///
    /// * See [`Self::setup_with_context()`]
    pub async fn setup(test_dir: TempDir) -> Result<Self, Error> {
        Self::setup_with_context(test_dir, ()).await
    }
}

impl<Ctx> EigenlayerTestHarness<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
{
    /// Create a new `EigenlayerTestHarness` with a predefined context
    ///
    /// NOTE: If your context type depends on [`Self::env()`], see [`Self::setup()`]
    ///
    /// # Errors
    ///
    /// * TODO
    pub async fn setup_with_context(test_dir: TempDir, _context: Ctx) -> Result<Self, Error> {
        // Start local Anvil testnet
        let (container, http_endpoint, ws_endpoint) = start_default_anvil_testnet(true).await;

        // Setup Eigenlayer test environment
        let EigenlayerTestEnvironment {
            accounts,
            http_endpoint,
            ws_endpoint,
            eigenlayer_contract_addresses,
        } = setup_eigenlayer_test_environment(&http_endpoint, &ws_endpoint).await;

        // Setup temporary testing keystore
        let test_dir_path = test_dir.path().to_string_lossy().into_owned();
        inject_anvil_key(&test_dir, ANVIL_PRIVATE_KEYS[0])?;

        // Create context config
        let context_config = ContextConfig::create_eigenlayer_config(
            Url::parse(&http_endpoint)?,
            Url::parse(&ws_endpoint)?,
            test_dir_path,
            None,
            SupportedChains::LocalTestnet,
            eigenlayer_contract_addresses,
        );

        // Load environment
        let env = BlueprintEnvironment::load_with_config(context_config)
            .map_err(|e| Error::Setup(e.to_string()))?;

        // Create config
        let config = EigenlayerTestConfig {
            http_endpoint: Some(Url::parse(&http_endpoint)?),
            ws_endpoint: Some(Url::parse(&ws_endpoint)?),
            eigenlayer_contract_addresses: Some(eigenlayer_contract_addresses),
        };

        Ok(Self {
            env,
            config,
            http_endpoint: Url::parse(&http_endpoint)?,
            ws_endpoint: Url::parse(&ws_endpoint)?,
            accounts,
            eigenlayer_contract_addresses,
            _temp_dir: test_dir,
            _container: container,
            _phantom: core::marker::PhantomData,
        })
    }

    #[must_use]
    #[allow(clippy::used_underscore_binding)]
    pub fn set_context<Ctx2: Clone + Send + Sync + 'static>(
        self,
        _context: Ctx2,
    ) -> EigenlayerTestHarness<Ctx2> {
        EigenlayerTestHarness {
            env: self.env,
            config: self.config,
            http_endpoint: self.http_endpoint,
            ws_endpoint: self.ws_endpoint,
            accounts: self.accounts,
            eigenlayer_contract_addresses: self.eigenlayer_contract_addresses,
            _temp_dir: self._temp_dir,
            _container: self._container,
            _phantom: PhantomData::<Ctx2>,
        }
    }

    #[must_use]
    pub fn env(&self) -> &BlueprintEnvironment {
        &self.env
    }
}

impl<Ctx> EigenlayerTestHarness<Ctx> {
    /// Gets a provider for the HTTP endpoint
    #[must_use]
    pub fn provider(&self) -> RootProvider {
        get_provider_http(self.http_endpoint.as_str())
    }

    /// Gets the list of accounts
    #[must_use]
    pub fn accounts(&self) -> &[Address] {
        &self.accounts
    }

    /// Gets the owner account (first account)
    #[must_use]
    pub fn owner_account(&self) -> Address {
        self.accounts[1]
    }

    /// Gets the aggregator account (ninth account)
    #[must_use]
    pub fn aggregator_account(&self) -> Address {
        self.accounts[9]
    }

    /// Gets the task generator account (fourth account)
    #[must_use]
    pub fn task_generator_account(&self) -> Address {
        self.accounts[4]
    }
}
