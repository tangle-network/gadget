use crate::Error;
use crate::env::{EigenlayerTestEnvironment, setup_eigenlayer_test_environment};
use alloy_primitives::Address;
use alloy_provider::RootProvider;
use blueprint_evm_extra::util::get_provider_http;
use blueprint_runner::config::{BlueprintEnvironment, ContextConfig, SupportedChains};
use blueprint_runner::eigenlayer::config::EigenlayerProtocolSettings;
use gadget_chain_setup::anvil::keys::{ANVIL_PRIVATE_KEYS, inject_anvil_key};
use gadget_chain_setup::anvil::{Container, start_default_anvil_testnet};
use gadget_core_testing_utils::harness::{BaseTestHarness, TestHarness};
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
    base: BaseTestHarness<EigenlayerTestConfig>,
    pub http_endpoint: Url,
    pub ws_endpoint: Url,
    pub accounts: Vec<Address>,
    pub eigenlayer_contract_addresses: EigenlayerProtocolSettings,
    _temp_dir: tempfile::TempDir,
    _container: Container,
    _phantom: core::marker::PhantomData<Ctx>,
}

impl<Ctx> TestHarness for EigenlayerTestHarness<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
{
    type Config = EigenlayerTestConfig;
    type Context = Ctx;
    type Error = Error;

    async fn setup(test_dir: TempDir, _context: Self::Context) -> Result<Self, Self::Error> {
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
        inject_anvil_key(&test_dir, ANVIL_PRIVATE_KEYS[0]).unwrap();

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

        let base = BaseTestHarness::new(env, config);

        Ok(Self {
            base,
            http_endpoint: Url::parse(&http_endpoint)?,
            ws_endpoint: Url::parse(&ws_endpoint)?,
            accounts,
            eigenlayer_contract_addresses,
            _temp_dir: test_dir,
            _container: container,
            _phantom: core::marker::PhantomData,
        })
    }

    fn env(&self) -> &BlueprintEnvironment {
        &self.base.env
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
