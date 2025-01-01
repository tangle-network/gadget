use subxt::client::OnlineClientT;
use subxt::error::TransactionError;
use subxt::tx::{TxInBlock, TxStatus};
use tangle_subxt::subxt;

/// Extension trait for transaction progress handling.
///
/// This trait provides additional methods for handling the progress of a transaction,
/// such as waiting for the transaction to be included in a block successfully.
///
/// # Type Parameters
///
/// - `T`: The configuration type for the Substrate runtime.
/// - `C`: The client type that implements the `OnlineClientT` trait.
#[async_trait::async_trait]
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
    async fn wait_for_in_block(mut self) -> Result<TxInBlock<T, C>, subxt::Error>;

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

#[async_trait::async_trait]
impl<T: subxt::Config, C: OnlineClientT<T>> TxProgressExt<T, C> for subxt::tx::TxProgress<T, C> {
    async fn wait_for_in_block(mut self) -> Result<TxInBlock<T, C>, subxt::Error> {
        while let Some(status) = self.next().await {
            match status? {
                // In Block! Return.
                TxStatus::InBestBlock(s) => return Ok(s),
                // Error scenarios; return the error.
                TxStatus::Error { message } => return Err(TransactionError::Error(message).into()),
                TxStatus::Invalid { message } => {
                    return Err(TransactionError::Invalid(message).into())
                }
                TxStatus::Dropped { message } => {
                    return Err(TransactionError::Dropped(message).into())
                }
                // Ignore and wait for next status event:
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
