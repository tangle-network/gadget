use crate::{MyContext, XsquareEventHandler};
use blueprint_sdk::logging::setup_log;
use blueprint_sdk::testing::tempfile;
use blueprint_sdk::testing::utils::harness::TestHarness;
use blueprint_sdk::testing::utils::runner::TestEnv;
use blueprint_sdk::testing::utils::tangle::{InputValue, OutputValue, TangleTestHarness};
use color_eyre::Result;
use std::time::Duration;

#[tokio::test]
async fn test_incredible_squaring() -> Result<()> {
    color_eyre::install()?;
    setup_log();

    // Initialize test harness (node, keys, deployment)
    let temp_dir = tempfile::TempDir::new()?;
    let harness = TangleTestHarness::setup(temp_dir).await?;
    let env = harness.env().clone();

    // Create blueprint-specific context
    let blueprint_ctx = MyContext {
        env: env.clone(),
        call_id: None,
    };

    // Initialize event handler
    let handler = XsquareEventHandler::new(&env.clone(), blueprint_ctx)
        .await
        .unwrap();

    // Setup service
    let (mut test_env, service_id, _blueprint_id) = harness.setup_services(false).await?;
    test_env.add_job(handler);

    test_env.run_runner().await.unwrap();

    // Execute job and verify result
    let results = harness
        .execute_job(
            service_id,
            0,
            vec![InputValue::Uint64(5)],
            vec![OutputValue::Uint64(25)],
        )
        .await?;

    assert_eq!(results.service_id, service_id);
    Ok(())
}

#[tokio::test]
async fn test_pre_register_incredible_squaring() -> Result<()> {
    setup_log();

    // Initialize test harness (node, keys, deployment)
    let temp_dir = tempfile::TempDir::new()?;
    let harness = TangleTestHarness::setup(temp_dir).await?;
    let env = harness.env().clone();

    // Create blueprint-specific context
    let blueprint_ctx = MyContext {
        env: env.clone(),
        call_id: None,
    };

    // Initialize event handler
    let handler = XsquareEventHandler::new(&env.clone(), blueprint_ctx)
        .await
        .unwrap();

    // Setup service, but we don't register yet
    let (mut test_env, _, blueprint_id) = harness.setup_services(true).await?;
    test_env.add_job(handler);

    // Run once for pre-registration
    test_env.run_runner().await.unwrap();

    tokio::time::sleep(Duration::from_secs(2)).await;

    let service_id = harness.request_service(blueprint_id).await.unwrap();

    // Run again to actually run the service, now that we have registered
    test_env.run_runner().await.unwrap();

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Execute job and verify result
    let _results = harness
        .execute_job(
            service_id,
            0,
            vec![InputValue::Uint64(5)],
            vec![OutputValue::Uint64(25)],
        )
        .await?;

    Ok(())
}
