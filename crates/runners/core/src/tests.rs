use crate::config::BlueprintConfig;
use crate::error::RunnerError as Error;
use crate::runner::{BackgroundService, BlueprintRunner};
use gadget_config::GadgetConfiguration;
use tokio::sync::oneshot;

struct MockBlueprintConfig;

#[async_trait::async_trait]
impl BlueprintConfig for MockBlueprintConfig {
    async fn requires_registration(&self, _env: &GadgetConfiguration) -> Result<bool, Error> {
        Ok(false)
    }

    async fn register(&self, _env: &GadgetConfiguration) -> Result<(), Error> {
        Ok(())
    }
}

struct MockBackgroundService;

#[async_trait::async_trait]
impl BackgroundService for MockBackgroundService {
    async fn start(&self) -> Result<oneshot::Receiver<Result<(), Error>>, Error> {
        let (tx, rx) = oneshot::channel();
        tokio::spawn(async move {
            let _ = tx.send(Ok(()));
        });
        Ok(rx)
    }
}

#[tokio::test]
async fn test_runner_creation() {
    let config = MockBlueprintConfig;
    let env = GadgetConfiguration::default();
    let runner = BlueprintRunner::new(config, env);

    assert!(runner.jobs.is_empty());
    assert!(runner.background_services.is_empty());
}

#[tokio::test]
async fn test_background_service_addition() {
    let config = MockBlueprintConfig;
    let env = GadgetConfiguration::default();
    let mut runner = BlueprintRunner::new(config, env);

    runner.background_service(Box::new(MockBackgroundService));
    assert_eq!(runner.background_services.len(), 1);
}

#[tokio::test]
async fn test_runner_execution() {
    let config = MockBlueprintConfig;
    let env = GadgetConfiguration::default();
    let mut runner = BlueprintRunner::new(config, env);

    runner.background_service(Box::new(MockBackgroundService));
    let result = runner.run().await;
    assert!(result.is_ok());
}
