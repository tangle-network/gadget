/// Send a transaction to the Tangle network.
///
/// # Errors
///
/// Returns a [`subxt::Error`] if the transaction fails.
#[tracing::instrument(skip_all)]
pub async fn send<T, S, X>(
    client: &subxt::OnlineClient<T>,
    signer: &S,
    xt: &X,
) -> Result<subxt::blocks::ExtrinsicEvents<T>, subxt::Error>
where
    T: subxt::Config,
    S: subxt::tx::Signer<T>,
    X: subxt::tx::Payload,
    <T::ExtrinsicParams as subxt::config::ExtrinsicParams<T>>::Params: Default,
{
    if let Some(details) = xt.validation_details() {
        gadget_logging::debug!("Calling {}.{}", details.pallet_name, details.call_name);
    }

    gadget_logging::debug!("Waiting for the transaction to be included in a finalized block");
    let progress = client
        .tx()
        .sign_and_submit_then_watch_default(xt, signer)
        .await?;

    #[cfg(not(test))]
    {
        gadget_logging::debug!("Waiting for finalized success ...");
        let result = progress.wait_for_finalized_success().await?;
        gadget_logging::debug!(
            "Transaction with hash: {:?} has been finalized",
            result.extrinsic_hash()
        );
        Ok(result)
    }
    #[cfg(test)]
    {
        use crate::tx_progress::TxProgressExt;

        // In tests, we don't wait for the transaction to be finalized.
        // This is because the test environment we will be using instant sealing.
        // Instead, we just wait for the transaction to be included in a block.
        gadget_logging::debug!("Waiting for in block success ...");
        let result = progress.wait_for_in_block_success().await?;
        gadget_logging::debug!(
            "Transaction with hash: {:?} has been included in a block",
            result.extrinsic_hash()
        );
        Ok(result)
    }
}
