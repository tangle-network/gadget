use std::marker::PhantomData;
use std::path::PathBuf;
use async_trait::async_trait;
use futures::Future;
use std::pin::Pin;
use tempfile::TempDir;
use tokio::task::JoinHandle;
use tracing::{info, error};
use thiserror::Error;

use gadget_config::GadgetConfiguration;
use gadget_runners::core::{
    config::BlueprintConfig,
    runner::BlueprintRunner,
    error::RunnerError,
};

/// Trait defining platform-specific runner configuration and setup
#[async_trait]
pub trait PlatformConfig: BlueprintConfig + Send + Sync + 'static {
    /// Platform-specific initialization
    async fn init(&self, env: &GadgetConfiguration) -> Result<(), TestRunnerError>;
    
    /// Platform-specific cleanup
    async fn cleanup(&self) -> Result<(), TestRunnerError>;
}

/// Generic job trait that all test jobs must implement
#[async_trait]
pub trait TestJob: Send + Sync + 'static {
    type Output;
    type Error: std::error::Error + Send + Sync + 'static;
    
    async fn execute(&self, runner: &mut BlueprintRunner) -> Result<Self::Output, Self::Error>;
}

/// Generic test runner that can be specialized for different platforms
pub struct GenericTestRunner<P: PlatformConfig, J: TestJob> {
    temp_dir: TempDir,
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
        let mut runner = BlueprintRunner::new(
            self.platform_config.clone(),
            self.config.clone(),
        );
        
        job.execute(&mut runner)
            .await
            .map_err(|e| TestRunnerError::ExecutionError(e.to_string()))
    }
    
    /// Run multiple jobs concurrently
    pub fn run_jobs_concurrent(
        &self,
        jobs: Vec<J>,
    ) -> Vec<JoinHandle<Result<J::Output, TestRunnerError>>> {
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