use gadget_config::GadgetConfiguration;
use tempfile::TempDir;

/// Generic test harness trait that defines common functionality for all test harnesses
#[async_trait::async_trait]
pub trait TestHarness {
    /// The configuration type used by this harness
    type Config;
    /// The error type returned by this harness
    type Error;

    /// Creates a new test harness with the given configuration
    async fn setup(test_dir: TempDir) -> Result<Self, Self::Error>
    where
        Self: Sized;

    /// Gets the gadget configuration
    fn env(&self) -> &GadgetConfiguration;

    /// Gets the configuration
    fn config(&self) -> &Self::Config;
}

/// Base implementation of a test harness that can be used by specific implementations
pub struct BaseTestHarness<C> {
    /// The environment configuration
    pub env: GadgetConfiguration,
    /// The harness-specific configuration
    pub config: C,
}

impl<C> BaseTestHarness<C> {
    /// Creates a new base test harness
    pub fn new(env: GadgetConfiguration, config: C) -> Self {
        Self { env, config }
    }
}
