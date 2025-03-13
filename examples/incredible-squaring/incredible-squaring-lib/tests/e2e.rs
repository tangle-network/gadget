use blueprint_sdk::Job;
use blueprint_sdk::testing::utils::setup_log;
use blueprint_sdk::tangle::layers::TangleLayer;
use blueprint_sdk::testing::tempfile;
use blueprint_sdk::testing::utils::tangle::{InputValue, OutputValue, TangleTestHarness};
use color_eyre::Result;
use incredible_squaring_blueprint_lib::square;

#[tokio::test]
async fn test_incredible_squaring() -> Result<()> {
    color_eyre::install()?;
    setup_log();

    // Initialize test harness (node, keys, deployment)
    let temp_dir = tempfile::TempDir::new()?;
    let harness = TangleTestHarness::setup(temp_dir).await?;

    // Setup service
    let (mut test_env, service_id, _blueprint_id) = harness.setup_services::<1>(false).await?;
    test_env.initialize().await?;

    // Add the job to the node, and start it
    test_env.add_job(square.layer(TangleLayer)).await;
    test_env.start().await?;

    // Submit job and wait for execution
    let job = harness
        .submit_job(service_id, 0, vec![InputValue::Uint64(5)])
        .await?;
    let results = harness.wait_for_job_execution(service_id, job).await?;

    // Verify results match expected output
    harness.verify_job(&results, vec![OutputValue::Uint64(25)]);

    assert_eq!(results.service_id, service_id);
    Ok(())
}
