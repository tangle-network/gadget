use crate::logger::Logger;

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
    logger: &Logger,
) -> Result<subxt::blocks::ExtrinsicEvents<T>, subxt::Error>
where
    T: subxt::Config,
    S: subxt::tx::Signer<T>,
    X: subxt::tx::Payload,
    <T::ExtrinsicParams as subxt::config::ExtrinsicParams<T>>::Params: Default,
{
    if let Some(details) = xt.validation_details() {
        logger.debug(format!(
            "Calling {}.{}",
            details.pallet_name, details.call_name
        ));
    }

    logger.debug("Waiting for the transaction to be included in a finalized block");
    let progress = client
        .tx()
        .sign_and_submit_then_watch_default(xt, signer)
        .await?;

    logger.debug("Waiting for finalized success ...");
    let result = progress.wait_for_finalized_success().await?;
    logger.debug(format!(
        "Transaction with hash: {:?} has been finalized",
        result.extrinsic_hash()
    ));
    Ok(result)
}
