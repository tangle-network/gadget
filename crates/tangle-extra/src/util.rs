use alloc::sync::Arc;
use tangle_subxt::subxt;

/// Send a transaction to the Tangle network.
///
/// # Errors
///
/// Returns a [`subxt::Error`] if the transaction fails.
pub async fn send<T, S, X>(
    client: Arc<subxt::OnlineClient<T>>,
    signer: Arc<S>,
    xt: X,
) -> Result<subxt::blocks::ExtrinsicEvents<T>, subxt::Error>
where
    T: subxt::Config,
    S: subxt::tx::Signer<T>,
    X: subxt::tx::Payload,
    <T::ExtrinsicParams as subxt::config::ExtrinsicParams<T>>::Params: Default,
{
    #[cfg(feature = "tracing")]
    if let Some(details) = xt.validation_details() {
        blueprint_core::debug!("Calling {}.{}", details.pallet_name, details.call_name);
    }

    blueprint_core::debug!("Waiting for the transaction to be included in a finalized block");
    let progress = client
        .tx()
        .sign_and_submit_then_watch_default(&xt, &*signer)
        .await?;

    #[cfg(not(test))]
    {
        blueprint_core::debug!("Waiting for finalized success ...");
        let result = progress.wait_for_finalized_success().await?;
        blueprint_core::debug!(
            "Transaction with hash: {:?} has been finalized",
            result.extrinsic_hash()
        );
        Ok(result)
    }
    #[cfg(test)]
    {
        use gadget_utils::tangle::tx_progress::TxProgressExt;

        // In tests, we don't wait for the transaction to be finalized.
        // This is because the test environment we will be using instant sealing.
        // Instead, we just wait for the transaction to be included in a block.
        blueprint_core::debug!("Waiting for in block success ...");
        let result = progress.wait_for_in_block_success().await?;
        blueprint_core::debug!(
            "Transaction with hash: {:?} has been included in a block",
            result.extrinsic_hash()
        );
        Ok(result)
    }
}
