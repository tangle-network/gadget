use crate::util::TxProgressExt;
use alloc::sync::Arc;
use gadget_testing_utils::tangle::TangleTestHarness;
use tangle_subxt::subxt::tx::Signer;

#[tokio::test]
async fn test_transaction_submission() -> color_eyre::Result<()> {
    // Setup test harness
    let test_dir = tempfile::TempDir::new()?;
    let harness = TangleTestHarness::<()>::setup(test_dir).await?;

    // Test basic transaction submission
    let tx = tangle_subxt::tangle_testnet_runtime::api::tx()
        .balances()
        .transfer_keep_alive(
            tangle_subxt::subxt::utils::MultiAddress::Id(
                harness.sr25519_signer.account_id().clone(),
            ),
            1_000,
        );

    let result = crate::util::send(
        Arc::new(harness.client().subxt_client().clone()),
        Arc::new(harness.sr25519_signer),
        tx,
    )
    .await;
    assert!(result.is_ok(), "Transaction submission should succeed");
    Ok(())
}

#[tokio::test]
async fn test_transaction_progress_tracking() -> color_eyre::Result<()> {
    // Setup test harness
    let test_dir = tempfile::TempDir::new()?;
    let harness = TangleTestHarness::<()>::setup(test_dir).await?;

    // Submit transaction and track progress
    let tx = tangle_subxt::tangle_testnet_runtime::api::tx()
        .balances()
        .transfer_keep_alive(
            tangle_subxt::subxt::utils::MultiAddress::Id(
                harness.sr25519_signer.account_id().clone(),
            ),
            1_000,
        );

    let tx_progress = harness
        .client()
        .subxt_client()
        .tx()
        .sign_and_submit_then_watch_default(&tx, &harness.sr25519_signer.clone())
        .await?;
    let result = tx_progress.wait_for_in_block_success().await;
    assert!(result.is_ok(), "Transaction should reach in-block state");
    Ok(())
}
