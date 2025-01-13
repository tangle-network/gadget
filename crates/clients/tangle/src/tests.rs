use super::*;
use crate::client::TangleClient;
use color_eyre::Result;
use gadget_client_core::GadgetServicesClient;
use gadget_config::GadgetConfiguration;
use gadget_tangle_testing_utils::TangleTestHarness;
use subxt::tx::Signer;

async fn setup_test_harness() -> Result<(TangleTestHarness, TangleClient)> {
    let harness = TangleTestHarness::setup().await?;
    let config = GadgetConfiguration::default();
    let client = TangleClient::new(config).await?;
    Ok((harness, client))
}

#[tokio::test]
async fn test_client_initialization() -> Result<()> {
    let (_harness, client) = setup_test_harness().await?;

    // Test basic client functionality
    assert!(
        client.subxt_client().blocks().at_latest().await.is_ok(),
        "Client should be connected"
    );
    assert!(
        client.account_id().0.len() == 32,
        "Account ID should be valid"
    );

    Ok(())
}

#[tokio::test]
async fn test_operator_metadata() -> Result<()> {
    let (harness, client) = setup_test_harness().await?;

    // Get operator metadata for the test account
    let metadata = client
        .operator_metadata(harness.sr25519_signer.account_id().clone())
        .await?;
    assert!(
        metadata.is_none(),
        "New account should not have operator metadata"
    );

    Ok(())
}

#[tokio::test]
async fn test_services_client() -> Result<()> {
    let (harness, client) = setup_test_harness().await?;
    let services = client.services_client();

    // Test blueprint queries
    let block_hash = client.now().await.expect("Should get current block hash");

    // Query non-existent blueprint
    let blueprint = services.get_blueprint_by_id(block_hash, 999999).await?;
    assert!(
        blueprint.is_none(),
        "Non-existent blueprint should return None"
    );

    // Query operator blueprints
    let blueprints = services
        .query_operator_blueprints(block_hash, harness.sr25519_signer.account_id().clone())
        .await?;
    assert!(
        blueprints.is_empty(),
        "New operator should have no blueprints"
    );

    Ok(())
}

#[tokio::test]
async fn test_events_client() -> Result<()> {
    let (_harness, client) = setup_test_harness().await?;

    // Test event subscription
    let latest = client.latest_event().await;
    assert!(latest.is_some(), "Should have access to latest event");

    // Test event stream
    let event = client.next_event().await;
    assert!(event.is_some(), "Should be able to get next event");

    if let Some(event) = event {
        assert!(event.number > 0, "Block number should be positive");
        assert!(event.hash != [0u8; 32], "Block hash should not be zero");
    }

    Ok(())
}

#[tokio::test]
async fn test_gadget_services_client() -> Result<()> {
    let (_harness, client) = setup_test_harness().await?;

    // Test operator set retrieval
    let operators = client.get_operators().await?;
    assert!(!operators.is_empty(), "Should have at least one operator");

    // Test operator ID retrieval
    let operator_id = client.operator_id().await?;
    assert!(
        operator_id.0.len() == 33,
        "Operator ID should be valid ECDSA public key"
    );

    // Test blueprint ID retrieval
    let blueprint_id = client.blueprint_id().await?;
    assert!(blueprint_id > 0, "Blueprint ID should be positive");

    Ok(())
}

#[tokio::test]
async fn test_service_operators() -> Result<()> {
    let (_harness, client) = setup_test_harness().await?;
    let services = client.services_client();

    // Get current block hash
    let block_hash = client.now().await.expect("Should get current block hash");

    // Query service operators for a non-existent service
    let operators = services
        .current_service_operators(block_hash, 999999)
        .await?;
    assert!(
        operators.is_empty(),
        "Non-existent service should have no operators"
    );

    Ok(())
}
