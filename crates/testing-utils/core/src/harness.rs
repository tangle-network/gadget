use blueprint_runner::config::BlueprintEnvironment;
use tempfile::TempDir;

/// Generic test harness trait that defines common functionality for all test harnesses
#[async_trait::async_trait]
pub trait TestHarness {
    /// The configuration type used by this harness
    type Config;
    /// The context type passed down to each job
    type Context: Clone + Send + Sync + 'static;

    /// The error type returned by this harness
    type Error;

    /// Creates a new test harness with the given configuration
    async fn setup(test_dir: TempDir, context: Self::Context) -> Result<Self, Self::Error>
    where
        Self: Sized;

    /// Gets the gadget configuration
    fn env(&self) -> &BlueprintEnvironment;
}

/// Base implementation of a test harness that can be used by specific implementations
pub struct BaseTestHarness<C> {
    /// The environment configuration
    pub env: BlueprintEnvironment,
    /// The harness-specific configuration
    pub config: C,
}

impl<C> BaseTestHarness<C> {
    /// Creates a new base test harness
    pub fn new(env: BlueprintEnvironment, config: C) -> Self {
        Self { env, config }
    }
}
