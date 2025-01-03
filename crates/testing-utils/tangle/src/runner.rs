use async_trait::async_trait;

use gadget_config::GadgetConfiguration;
use gadget_core_testing_utils::{PlatformConfig, TestRunnerError};
use gadget_runners::core::config::BlueprintConfig;
use gadget_runners::tangle::tangle::TangleConfig;

// Newtype wrapper around TangleConfig to implement external traits
#[derive(Clone)]
pub struct TangleTestConfig(TangleConfig);

impl From<TangleConfig> for TangleTestConfig {
    fn from(config: TangleConfig) -> Self {
        TangleTestConfig(config)
    }
}

impl std::ops::Deref for TangleTestConfig {
    type Target = TangleConfig;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait]
impl PlatformConfig for TangleTestConfig {
    async fn init(&self, env: &GadgetConfiguration) -> Result<(), TestRunnerError> {
        if self
            .requires_registration(env)
            .await
            .map_err(|e| TestRunnerError::SetupError(e.to_string()))?
        {
            self.register(env)
                .await
                .map_err(|e| TestRunnerError::SetupError(e.to_string()))?;
        }
        Ok(())
    }

    async fn cleanup(&self) -> Result<(), TestRunnerError> {
        Ok(())
    }
}

#[async_trait]
impl BlueprintConfig for TangleTestConfig {
    async fn register(
        &self,
        env: &GadgetConfiguration,
    ) -> Result<(), gadget_runners::core::error::RunnerError> {
        self.0.register(env).await
    }

    async fn requires_registration(
        &self,
        env: &GadgetConfiguration,
    ) -> Result<bool, gadget_runners::core::error::RunnerError> {
        self.0.requires_registration(env).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gadget_core_testing_utils::{GenericTestRunner, TestJob};
    use gadget_runners::core::runner::BlueprintRunner;
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
        let runner = GenericTestRunner::<TangleTestConfig, MockJob>::new(
            TangleTestConfig::from(TangleConfig::default()),
            None,
        )
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
        let runner = GenericTestRunner::<TangleTestConfig, MockJob>::new(
            TangleTestConfig::from(TangleConfig::default()),
            None,
        )
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
