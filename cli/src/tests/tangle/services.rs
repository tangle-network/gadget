// use color_eyre::eyre::Result;
// use gadget_logging::setup_log;
// use gadget_testing_utils::{harness::TestHarness, tangle::TangleTestHarness};
//
// use crate::commands;
//
// #[tokio::test]
// async fn test_list_requests() -> Result<()> {
//     color_eyre::install()?;
//     setup_log();
//
//     let temp_dir = tempfile::TempDir::new()?;
//     let harness = TangleTestHarness::setup(temp_dir).await?;
//     let (mut test_env, _service_id, _blueprint_id) = harness.setup_services::<1>(false).await?;
//     test_env.initialize().await?;
//
//     let nodes = test_env.node_handles().await;
//     let alice_node = &nodes[0];
//     let env = alice_node.gadget_config().await;
//
//     let result = commands::list_requests(env.ws_rpc_endpoint.clone()).await;
//
//     assert!(result.is_ok());
//     Ok(())
// }
//
// #[tokio::test]
// async fn test_request_service() -> Result<()> {
//     color_eyre::install()?;
//     setup_log();
//
//     let temp_dir = tempfile::TempDir::new()?;
//     let harness = TangleTestHarness::setup(temp_dir).await?;
//     let (mut test_env, _service_id, blueprint_id) = harness.setup_services::<1>(false).await?;
//     test_env.initialize().await?;
//
//     let nodes = test_env.node_handles().await;
//     let alice_node = &nodes[0];
//     let env = alice_node.gadget_config().await;
//
//     let result = commands::request_service(
//         env.ws_rpc_endpoint.clone(),
//         blueprint_id,
//         10,
//         20,
//         vec![],
//         1000,
//         env.keystore_uri.clone(),
//     )
//     .await;
//
//     assert!(result.is_ok());
//     Ok(())
// }
//
// #[tokio::test]
// async fn test_submit_job() -> Result<()> {
//     color_eyre::install()?;
//     setup_log();
//
//     let temp_dir = tempfile::TempDir::new()?;
//     let harness = TangleTestHarness::setup(temp_dir).await?;
//     let (mut test_env, service_id, blueprint_id) = harness.setup_services::<1>(false).await?;
//     test_env.initialize().await?;
//
//     let nodes = test_env.node_handles().await;
//     let alice_node = &nodes[0];
//     let env = alice_node.gadget_config().await;
//
//     let result = commands::submit_job(
//         env.ws_rpc_endpoint.clone(),
//         Some(service_id),
//         blueprint_id,
//         env.keystore_uri.clone(),
//         1,
//         None,
//     )
//     .await;
//
//     assert!(result.is_ok());
//     Ok(())
// }
//
// #[tokio::test]
// async fn test_accept_request() -> Result<()> {
//     color_eyre::install()?;
//     setup_log();
//
//     let temp_dir = tempfile::TempDir::new()?;
//     let harness = TangleTestHarness::setup(temp_dir).await?;
//     let (mut test_env, _service_id, blueprint_id) = harness.setup_services::<1>(false).await?;
//     test_env.initialize().await?;
//
//     let nodes = test_env.node_handles().await;
//     let alice_node = &nodes[0];
//     let env = alice_node.gadget_config().await;
//
//     let request_result = commands::request_service(
//         env.ws_rpc_endpoint.clone(),
//         blueprint_id,
//         10,
//         20,
//         vec![],
//         1000,
//         env.keystore_uri.clone(),
//     )
//     .await;
//     assert!(request_result.is_ok());
//
//     let result = commands::accept_request(
//         env.ws_rpc_endpoint.clone(),
//         10,
//         20,
//         15,
//         env.keystore_uri.clone(),
//         1,
//     )
//     .await;
//
//     assert!(result.is_ok());
//     Ok(())
// }
//
// #[tokio::test]
// async fn test_reject_request() -> Result<()> {
//     color_eyre::install()?;
//     setup_log();
//
//     let temp_dir = tempfile::TempDir::new()?;
//     let harness = TangleTestHarness::setup(temp_dir).await?;
//     let (mut test_env, _service_id, blueprint_id) = harness.setup_services::<1>(false).await?;
//     test_env.initialize().await?;
//
//     let nodes = test_env.node_handles().await;
//     let alice_node = &nodes[0];
//     let env = alice_node.gadget_config().await;
//
//     let request_result = commands::request_service(
//         env.ws_rpc_endpoint.clone(),
//         blueprint_id,
//         10,
//         20,
//         vec![],
//         1000,
//         env.keystore_uri.clone(),
//     )
//     .await;
//     assert!(request_result.is_ok());
//
//     let result =
//         commands::reject_request(env.ws_rpc_endpoint.clone(), env.keystore_uri.clone(), 1).await;
//
//     assert!(result.is_ok());
//     Ok(())
// }
