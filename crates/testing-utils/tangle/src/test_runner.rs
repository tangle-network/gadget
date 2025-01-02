use crate::error::Error;
use futures::Future;
use gadget_config::GadgetConfiguration;
use gadget_runners::core::config::BlueprintConfig;
use gadget_runners::core::runner::BlueprintRunner;
use std::path::PathBuf;
use std::pin::Pin;
use tempfile::TempDir;
use tokio::task::JoinHandle;
use tracing::info;

#[allow(clippy::type_complexity)]
pub struct RunnerSetup<C: BlueprintConfig> {
    pub config: C,
    pub setup: Box<
        dyn FnOnce(&mut BlueprintRunner) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send>>
            + Send,
    >,
}

impl<C: BlueprintConfig> RunnerSetup<C> {
    pub fn new<F, Fut>(config: C, setup: F) -> Self
    where
        F: FnOnce(&mut BlueprintRunner) -> Fut + Send + 'static,
        Fut: Future<Output = Result<(), Error>> + Send + 'static,
    {
        RunnerSetup {
            config,
            setup: Box::new(move |runner| Box::pin(setup(runner))),
        }
    }
}

/// A test environment for running blueprint runners directly
pub struct RunnerTestEnv {
    /// Temporary directory for test data
    pub temp_dir: TempDir,
    /// Runner configuration
    pub config: GadgetConfiguration,
}

impl RunnerTestEnv {
    /// Create a new test environment with default configuration
    pub fn new() -> Result<Self, Error> {
        let temp_dir = tempfile::tempdir()?;
        let config = GadgetConfiguration::default();

        Ok(Self { temp_dir, config })
    }

    /// Create a new test environment with custom configuration
    pub fn with_config(config: GadgetConfiguration) -> Result<Self, Error> {
        let temp_dir = tempfile::tempdir()?;

        Ok(Self { temp_dir, config })
    }

    /// Get the path to the temporary directory
    pub fn temp_dir_path(&self) -> PathBuf {
        self.temp_dir.path().to_path_buf()
    }

    /// Run a blueprint runner with the test environment configuration
    pub async fn run_runner<C, F, Fut>(&self, config: C, setup_fn: F) -> Result<(), Error>
    where
        C: BlueprintConfig + 'static,
        F: FnOnce(&mut BlueprintRunner) -> Fut,
        Fut: Future<Output = Result<(), Error>>,
    {
        info!("Setting up runner test environment");
        let mut runner = BlueprintRunner::new(config, self.config.clone());

        setup_fn(&mut runner).await?;

        info!("Starting runner");
        runner.run().await.map_err(Into::into)
    }

    /// Spawn multiple runners simultaneously
    pub fn spawn_runners<C, F, Fut>(
        &self,
        configs_and_setups: Vec<(C, F)>,
    ) -> Vec<JoinHandle<Result<(), Error>>>
    where
        C: BlueprintConfig + Send + 'static,
        F: FnOnce(&mut BlueprintRunner) -> Fut + Send + 'static,
        Fut: Future<Output = Result<(), Error>> + Send,
    {
        configs_and_setups
            .into_iter()
            .map(|(config, setup_fn)| {
                let config_clone = self.config.clone();
                tokio::spawn(async move {
                    let mut runner = BlueprintRunner::new(config, config_clone);
                    setup_fn(&mut runner).await?;
                    runner.run().await.map_err(Into::into)
                })
            })
            .collect()
    }

    /// Run multiple runners simultaneously and wait for all of them to complete
    pub async fn run_multiple_runners<C>(&self, setups: Vec<RunnerSetup<C>>) -> Result<(), Error>
    where
        C: BlueprintConfig + 'static,
    {
        for setup in setups {
            let mut runner = BlueprintRunner::new(setup.config, self.config.clone());
            (setup.setup)(&mut runner).await?;
            runner.run().await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use gadget_runners::core::config::BlueprintConfig;
    use gadget_runners::core::error::RunnerError;

    // Mock blueprint config for testing
    #[derive(Default, Clone)]
    struct MockConfig;

    #[async_trait]
    impl BlueprintConfig for MockConfig {
        async fn register(&self, _env: &GadgetConfiguration) -> Result<(), RunnerError> {
            Ok(())
        }

        async fn requires_registration(
            &self,
            _env: &GadgetConfiguration,
        ) -> Result<bool, RunnerError> {
            Ok(false)
        }
    }

    #[tokio::test]
    async fn test_single_runner() {
        let env = RunnerTestEnv::new().unwrap();

        env.run_runner(MockConfig::default(), |_runner| async { Ok(()) })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_multiple_runners() {
        let env = RunnerTestEnv::new().unwrap();

        let test_runner = |_runner: &mut BlueprintRunner| async { Ok(()) };

        let setups = vec![
            RunnerSetup::new(MockConfig::default(), test_runner),
            RunnerSetup::new(MockConfig::default(), test_runner),
            RunnerSetup::new(MockConfig::default(), test_runner),
        ];

        env.run_multiple_runners(setups).await.unwrap();
    }
}
