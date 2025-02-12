use crate::{MyContext, XsquareEventHandler};
use blueprint_sdk::config::GadgetConfiguration;
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

    // Setup service
    let (mut test_env, service_id, _blueprint_id) = harness.setup_services::<1>(false).await?;
    test_env.initialize().await?;

    let nodes = test_env.node_handles().await;
    let alice_node = &nodes[0];

    // Create blueprint-specific context
    let env = alice_node.gadget_config().await;
    let blueprint_ctx = MyContext {
        env: env.clone(),
        call_id: None,
    };

    // Initialize the event handler
    let handler = XsquareEventHandler::new(&env, blueprint_ctx).await?;

    // Add the job to the node, and start it
    alice_node.add_job(handler).await;
    test_env.start().await?;

    // Submit job and wait for execution
    let job = harness
        .submit_job(service_id, 0, vec![InputValue::Uint64(5)])
        .await?;
    let results = harness.wait_for_job_execution(service_id, job).await?;

    // Verify results match expected output
    let results = harness.verify_job(results, vec![OutputValue::Uint64(25)])?;

    assert_eq!(results.service_id, service_id);
    Ok(())
}

// #[tokio::test]
// async fn test_pre_registration_incredible_squaring() -> Result<()> {
//     setup_log();
//
//     // Initialize test harness (node, keys, deployment)
//     let temp_dir = tempfile::TempDir::new()?;
//     let harness = TangleTestHarness::setup(temp_dir).await?;
//
//     // Setup service, but we don't register yet
//     let (mut test_env, service_id, blueprint_id) = harness.setup_services::<1>(true).await?;
//     test_env.initialize().await?;
//
//     let nodes = test_env.node_handles().await;
//     let alice_node = &nodes[0];
//
//     // Create blueprint-specific context
//     let env = alice_node.gadget_config().await;
//     let blueprint_ctx = MyContext {
//         env: env.clone(),
//         call_id: None,
//     };
//
//     // Initialize the event handler
//     let handler = XsquareEventHandler::new(&env, blueprint_ctx).await?;
//
//     // Add the job to the node, and run once for pre-registration
//     alice_node.add_job(handler).await;
//     test_env.start().await?;
//
//     tokio::time::sleep(Duration::from_secs(2)).await;
//
//     let service_id = harness.request_service(blueprint_id).await.unwrap();
//
//     // Run again to actually run the service, now that we have registered
//     test_env.run_runner().await.unwrap();
//
//     tokio::time::sleep(Duration::from_secs(2)).await;
//
//     // Execute job and verify result
//     let _results = harness
//         .execute_job(
//             service_id,
//             0,
//             vec![InputValue::Uint64(5)],
//             vec![OutputValue::Uint64(25)],
//         )
//         .await?;
//
//     Ok(())
// }
