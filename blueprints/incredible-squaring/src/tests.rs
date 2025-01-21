use crate::{MyContext, XsquareEventHandler};
use color_eyre::Result;
use gadget_testing_utils::harness::TestHarness;
use gadget_testing_utils::runner::TestEnv;
use gadget_testing_utils::tangle::{InputValue, OutputValue, TangleTestHarness};

#[tokio::test]
async fn test_incredible_squaring() -> Result<()> {
    gadget_logging::setup_log();

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
    let (mut test_env, service_id) = harness.setup_services().await?;
    test_env.add_job(handler);

    tokio::spawn(async move {
        test_env.run_runner().await.unwrap();
    });

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
