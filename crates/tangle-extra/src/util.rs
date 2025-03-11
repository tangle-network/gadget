#[cfg(test)]
mod tests;

use alloc::sync::Arc;
use tangle_subxt::subxt;
use tangle_subxt::subxt::client::OnlineClientT;
use tangle_subxt::subxt::error::TransactionError;
use tangle_subxt::subxt::tx::{TxInBlock, TxStatus};

/// Extension trait for transaction progress handling.
///
/// This trait provides additional methods for handling the progress of a transaction,
/// such as waiting for the transaction to be included in a block successfully.
///
/// # Type Parameters
///
/// - `T`: The configuration type for the Substrate runtime.
/// - `C`: The client type that implements the `OnlineClientT` trait.
#[allow(async_fn_in_trait)]
pub trait TxProgressExt<T: subxt::Config, C> {
    /// Wait for the transaction to be in block, and return a [`TxInBlock`]
    /// instance when it is, or an error if there was a problem waiting for finalization.
    ///
    /// **Note:** consumes `self`. If you'd like to perform multiple actions as the state of the
    /// transaction progresses, use [`TxProgress::next()`] instead.
    ///
    /// **Note:** transaction statuses like `Invalid`/`Usurped`/`Dropped` indicate with some
    /// probability that the transaction will not make it into a block but there is no guarantee
    /// that this is true. In those cases the stream is closed however, so you currently have no way to find
    /// out if they finally made it into a block or not.
    async fn wait_for_in_block(self) -> Result<TxInBlock<T, C>, subxt::Error>;

    /// Wait for the transaction to be finalized, and for the transaction events to indicate
    /// that the transaction was successful. Returns the events associated with the transaction,
    /// as well as a couple of other details (block hash and extrinsic hash).
    ///
    /// **Note:** consumes self. If you'd like to perform multiple actions as progress is made,
    /// use [`TxProgress::next()`] instead.
    ///
    /// **Note:** transaction statuses like `Invalid`/`Usurped`/`Dropped` indicate with some
    /// probability that the transaction will not make it into a block but there is no guarantee
    /// that this is true. In those cases the stream is closed however, so you currently have no way to find
    /// out if they finally made it into a block or not.
    async fn wait_for_in_block_success(
        self,
    ) -> Result<subxt::blocks::ExtrinsicEvents<T>, subxt::Error>;
}

impl<T: subxt::Config, C: OnlineClientT<T>> TxProgressExt<T, C> for subxt::tx::TxProgress<T, C> {
    #[allow(clippy::needless_continue)]
    async fn wait_for_in_block(mut self) -> Result<TxInBlock<T, C>, subxt::Error> {
        while let Some(status) = self.next().await {
            match status? {
                // In Block! Return.
                TxStatus::InBestBlock(s) => return Ok(s),
                // Error scenarios; return the error.
                TxStatus::Error { message } => return Err(TransactionError::Error(message).into()),
                TxStatus::Invalid { message } => {
                    return Err(TransactionError::Invalid(message).into());
                }
                TxStatus::Dropped { message } => {
                    return Err(TransactionError::Dropped(message).into());
                }
                // Ignore and wait for next status event
                _ => continue,
            }
        }

        Err(subxt::error::RpcError::SubscriptionDropped.into())
    }

    async fn wait_for_in_block_success(
        self,
    ) -> Result<subxt::blocks::ExtrinsicEvents<T>, subxt::Error> {
        let evs = self.wait_for_in_block().await?.wait_for_success().await?;
        Ok(evs)
    }
}

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
