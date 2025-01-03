use async_trait::async_trait;
use std::marker::PhantomData;
use std::path::PathBuf;
use tokio::task::JoinHandle;
use tracing::error;

use gadget_config::GadgetConfiguration;
use gadget_runners::core::{config::BlueprintConfig, runner::BlueprintRunner};

use crate::error::TestRunnerError;

/// Trait defining platform-specific runner configuration and setup
#[async_trait]
pub trait PlatformConfig: BlueprintConfig + Clone + Send + Sync + 'static {
    /// Platform-specific initialization
    async fn init(&self, env: &GadgetConfiguration) -> Result<(), TestRunnerError>;

    /// Platform-specific cleanup
    async fn cleanup(&self) -> Result<(), TestRunnerError>;
}

/// Generic job trait that all test jobs must implement
#[async_trait]
pub trait TestJob: Send + Sync + 'static {
    /// The output type of the job execution
    type Output: Send + 'static;
    /// The error type that can occur during job execution
    type Error: std::error::Error + Send + Sync + 'static;

    /// Execute the job with the given runner
    async fn execute(&self, runner: &mut BlueprintRunner) -> Result<Self::Output, Self::Error>;
}

/// Generic test runner that can be specialized for different platforms
pub struct GenericTestRunner<P: PlatformConfig, J: TestJob> {
    temp_dir: tempfile::TempDir,
    config: GadgetConfiguration,
    platform_config: P,
    _job: PhantomData<J>,
}

impl<P: PlatformConfig, J: TestJob> GenericTestRunner<P, J> {
    /// Create a new test runner with the given configuration
    pub async fn new(
        platform_config: P,
        env_config: Option<GadgetConfiguration>,
    ) -> Result<Self, TestRunnerError> {
        let temp_dir = tempfile::tempdir()?;
        let config = env_config.unwrap_or_default();

        // Initialize platform-specific configuration
        platform_config.init(&config).await?;

        Ok(Self {
            temp_dir,
            config,
            platform_config,
            _job: PhantomData,
        })
    }

    /// Get the temporary directory path
    pub fn temp_dir_path(&self) -> PathBuf {
        self.temp_dir.path().to_path_buf()
    }

    /// Run a single job
    pub async fn run_job(&self, job: J) -> Result<J::Output, TestRunnerError> {
        let mut runner = BlueprintRunner::new(self.platform_config.clone(), self.config.clone());

        job.execute(&mut runner)
            .await
            .map_err(|e| TestRunnerError::ExecutionError(e.to_string()))
    }

    /// Run multiple jobs concurrently
    pub fn run_jobs_concurrent(
        &self,
        jobs: Vec<J>,
    ) -> Vec<JoinHandle<Result<J::Output, TestRunnerError>>>
    where
        J::Output: Send + 'static,
    {
        jobs.into_iter()
            .map(|job| {
                let config = self.config.clone();
                let platform_config = self.platform_config.clone();

                tokio::spawn(async move {
                    let mut runner = BlueprintRunner::new(platform_config, config);
                    job.execute(&mut runner)
                        .await
                        .map_err(|e| TestRunnerError::ExecutionError(e.to_string()))
                })
            })
            .collect()
    }
}

impl<P: PlatformConfig, J: TestJob> Drop for GenericTestRunner<P, J> {
    fn drop(&mut self) {
        // Run cleanup in a blocking task to ensure it completes
        let platform_config = self.platform_config.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                if let Err(e) = platform_config.cleanup().await {
                    error!("Failed to cleanup runner: {}", e);
                }
            })
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gadget_runners::core::error::RunnerError;
    use std::sync::Arc;

    // Mock implementation for testing
    #[derive(Clone)]
    struct MockJob {
        result: Arc<str>,
    }

    #[async_trait]
    impl TestJob for MockJob {
        type Output = String;
        type Error = TestRunnerError;

        async fn execute(
            &self,
            _runner: &mut BlueprintRunner,
        ) -> Result<Self::Output, Self::Error> {
            Ok(self.result.to_string())
        }
    }

    #[derive(Clone)]
    struct MockPlatformConfig;

    #[async_trait]
    impl PlatformConfig for MockPlatformConfig {
        async fn init(&self, _env: &GadgetConfiguration) -> Result<(), TestRunnerError> {
            Ok(())
        }

        async fn cleanup(&self) -> Result<(), TestRunnerError> {
            Ok(())
        }
    }

    #[async_trait]
    impl BlueprintConfig for MockPlatformConfig {
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
    async fn test_single_job() {
        let runner =
            GenericTestRunner::<MockPlatformConfig, MockJob>::new(MockPlatformConfig, None)
                .await
                .unwrap();

        let job = MockJob {
            result: Arc::from("test_result"),
        };

        let result = runner.run_job(job).await.unwrap();
        assert_eq!(result, "test_result");
    }

    #[tokio::test]
    async fn test_multiple_jobs() {
        let runner =
            GenericTestRunner::<MockPlatformConfig, MockJob>::new(MockPlatformConfig, None)
                .await
                .unwrap();

        let jobs = vec![
            MockJob {
                result: Arc::from("result1"),
            },
            MockJob {
                result: Arc::from("result2"),
            },
        ];

        let handles = runner.run_jobs_concurrent(jobs);

        let results = futures::future::join_all(handles)
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(results, vec!["result1", "result2"]);
    }
}
