use async_trait::async_trait;
use futures::Future;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::pin::Pin;
use tempfile::TempDir;
use thiserror::Error;
use tokio::task::JoinHandle;
use gadget_logging::{error, info};

use gadget_config::GadgetConfiguration;
use gadget_runners::core::{config::BlueprintConfig, error::RunnerError, runner::BlueprintRunner};
use gadget_runners::tangle::tangle::TangleConfig;
use gadget_core_testing_utils::runner::PlatformConfig;
use gadget_core_testing_utils::TestRunnerError;

#[async_trait]
impl PlatformConfig for TangleConfig {
    async fn init(&self, env: &GadgetConfiguration) -> Result<(), TestRunnerError> {
        // Tangle-specific initialization
        if self
            .requires_registration(env)
            .map_err(|e| TestRunnerError::SetupError(e.to_string()))?
        {
            self.register(env)
                .map_err(|e| TestRunnerError::SetupError(e.to_string()))?;
        }
        Ok(())
    }

    async fn cleanup(&self) -> Result<(), TestRunnerError> {
        // Tangle-specific cleanup if needed
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[tokio::test]
    async fn test_single_job() {
        let runner =
            GenericTestRunner::<tangle::TangleConfig, MockJob>::new(TangleConfig::default(), None)
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
            GenericTestRunner::<tangle::TangleConfig, MockJob>::new(TangleConfig::default(), None)
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
