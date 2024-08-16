use alloy_contract::ContractInstance;
use alloy_network::Ethereum;
use alloy_network::ReceiptResponse;
use alloy_primitives::FixedBytes;
use alloy_rpc_types::BlockNumberOrTag;
use core::{ops::Deref, time::Duration};

use std::sync::Arc;

use crate::events_watcher::ConstantWithMaxRetryCount;
use crate::store::LocalDatabase;
use alloy_network::Network;
use alloy_provider::Provider;
use alloy_rpc_types::{Filter, FilterBlockOption, Log};
use alloy_sol_types::SolEvent;
use alloy_transport::Transport;
use futures::TryFutureExt;

/// An error type for the event watcher.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// An error occurred in Alloy transport.
    #[error(transparent)]
    AlloyTransport(#[from] alloy_transport::TransportError),
    /// An error occurred in the backoff mechanism.
    #[error(transparent)]
    Backoff(#[from] backoff::Error<subxt::Error>),
    /// An error occurred in the event watcher and we need to restart it.
    #[error("An error occurred in the event watcher and we need to restart it.")]
    ForceRestart,
    /// An error occurred in the event handler.
    #[error(transparent)]
    Handler(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// A helper type to extract the [`EventHandler`] from the [`EventWatcher`] trait.
pub type EventHandlerFor<
    W,
    T: Transport + Clone + Send + Sync,
    P: Provider<T, N> + Send + Sync,
    N: Network + Send + Sync,
> = Box<
    dyn EventHandler<
            T,
            P,
            N,
            Contract = <W as EventWatcher<T, P, N>>::Contract,
            Events = <W as EventWatcher<T, P, N>>::Events,
        > + Send
        + Sync,
>;

/// A trait that defines a handler for a specific set of event types.
///
/// The handlers are implemented separately from the watchers, so that we can have
/// one event watcher and many event handlers that will run in parallel.
#[async_trait::async_trait]
pub trait EventHandler<T, P, N>: Send + Sync
where
    T: Transport + Clone + Send + Sync,
    P: Provider<T, N> + Send + Sync,
    N: Network + Send + Sync,
{
    /// The Type of contract this handler is for, Must be the same as the contract type in the
    /// watcher.
    type Contract: Deref<Target = alloy_contract::ContractInstance<T, P, N>> + Send + Sync;
    /// The type of event this handler is for.
    type Events: SolEvent + Clone + Send + Sync;

    /// a method to be called with the event information,
    /// it is up to the handler to decide what to do with the event.
    ///
    /// If this method returned an error, the handler will be considered as failed and will
    /// be discarded. to have a retry mechanism, use the [`EventHandlerWithRetry::handle_event_with_retry`] method
    /// which does exactly what it says.
    ///
    /// If this method returns Ok(true), the event will be marked as handled.
    async fn handle_event(
        &self,
        contract: &Self::Contract,
        (event, log): (Self::Events, Log),
    ) -> Result<(), Error>;

    /// Whether any of the events could be handled by the handler
    async fn can_handle_events(
        &self,
        (event, log): (Self::Events, Log),
        wrapper: &Self::Contract,
    ) -> Result<bool, Error>;
}

/// An Auxiliary trait to handle events with retry logic.
///
/// this trait is automatically implemented for all the event handlers.
#[async_trait::async_trait]
#[async_trait::async_trait]
pub trait EventHandlerWithRetry<T, P, N>: EventHandler<T, P, N> + Send + Sync
where
    T: Transport + Clone + Send + Sync,
    P: Provider<T, N> + Send + Sync,
    N: Network + Send + Sync,
    <Self as EventHandler<T, P, N>>::Events: 'async_trait,
{
    /// A method to be called with the event information,
    /// it is up to the handler to decide what to do with the event.
    ///
    /// If this method returned an error, the handler will be considered as failed and will
    /// be retried again, depends on the retry strategy. if you do not care about the retry
    /// strategy, use the [`EventHandler::handle_event`] method instead.
    ///
    /// If this method returns Ok(true), the event will be marked as handled.
    ///
    /// **Note**: this method is automatically implemented for all the event handlers.
    async fn handle_event_with_retry(
        &self,
        contract: &Self::Contract,
        (event, log): (Self::Events, Log),
        backoff: impl backoff::backoff::Backoff + Send + Sync + 'static,
    ) -> Result<(), Error> {
        if !self
            .can_handle_events((event.clone(), log.clone()), contract)
            .await?
        {
            return Ok(());
        };

        let wrapped_task = || {
            self.handle_event(contract, (event.clone(), log.clone()))
                .map_err(backoff::Error::transient)
        };
        backoff::future::retry(backoff, wrapped_task).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl<X, T, P, N> EventHandlerWithRetry<T, P, N> for X
where
    X: EventHandler<T, P, N> + Send + Sync,
    T: Transport + Clone + Send + Sync,
    P: Provider<T, N> + Send + Sync,
    N: Network + Send + Sync,
    <X as EventHandler<T, P, N>>::Events: 'async_trait,
{
}

/// A trait for watching events from a contract.
/// EventWatcher trait exists for deployments that are smart-contract / EVM based
#[async_trait::async_trait]
pub trait EventWatcher<T, P, N>: Send + Sync
where
    T: Transport + Clone + Send + Sync,
    P: Provider<T, N> + Send + Sync,
    N: Network + Send + Sync,
{
    /// A Helper tag used to identify the event watcher during the logs.
    const TAG: &'static str;
    /// The contract that this event watcher is watching.
    type Contract: Deref<Target = alloy_contract::ContractInstance<T, P, N>> + Send + Sync;
    /// The type of event this handler is for.
    type Events: SolEvent + Clone + Send + Sync;
    /// The genesis transaction hash for the contract.
    const GENESIS_TX_HASH: FixedBytes<32>;

    /// The Storage backend that will be used to store the required state for this event watcher
    /// Returns a task that should be running in the background
    /// that will watch events
    #[tracing::instrument(
        skip_all,
        fields(
            address = %contract.address(),
            tag = %Self::TAG,
        ),
    )]
    async fn run(
        &self,
        genesis_tx_hash: FixedBytes<32>,
        provider: Arc<P>,
        contract: Self::Contract,
        handlers: Vec<EventHandlerFor<Self, T, P, N>>,
    ) -> Result<(), Error> {
        let local_db = LocalDatabase::new("./db");
        let backoff = backoff::backoff::Constant::new(Duration::from_secs(1));
        let task = || async {
            let step = 100;
            let chain_id: u64 = provider
                .root()
                .get_chain_id()
                .map_err(Into::into)
                .map_err(backoff::Error::transient)
                .await?;

            // saves the last time we printed sync progress.
            let mut instant = std::time::Instant::now();
            // we only query this once, at the start of the events watcher.
            // then we will update it later once we fully synced.

            let mut target_block_number: u64 = provider
                .get_block_number()
                .map_err(Into::into)
                .map_err(backoff::Error::transient)
                .await?;

            local_db.set(
                &format!("TARGET_BLOCK_NUMBER_{}", contract.address()),
                target_block_number.into(),
            );

            let deployed_at = provider
                .get_transaction_receipt(Self::GENESIS_TX_HASH)
                .await
                .map_err(Into::into)?
                .map(|receipt| receipt.block_number().unwrap_or_default())
                .unwrap_or_default();

            loop {
                let block = local_db
                    .get(&format!("LAST_BLOCK_NUMBER_{}", contract.address()))
                    .unwrap_or_else(|| deployed_at);
                let dest_block = core::cmp::min(block + step, target_block_number);

                let events_filter = contract.event(
                    Filter::new()
                        .from_block(BlockNumberOrTag::Number(block + 1))
                        .to_block(BlockNumberOrTag::Number(dest_block)),
                );

                let events = events_filter.query().await.map_err(Into::into)?;
                let number_of_events = events.len();
                tracing::trace!("Found #{number_of_events} events");
                for (event, log) in events.into_iter() {
                    // Wraps each handler future in a retry logic, that will retry the handler
                    // if it fails, up to `MAX_RETRY_COUNT`, after this it will ignore that event for
                    // that specific handler.
                    const MAX_RETRY_COUNT: usize = 5;
                    let tasks = handlers.iter().map(|handler| {
                        // a constant backoff with maximum retry count is used here.
                        let backoff = ConstantWithMaxRetryCount::new(
                            Duration::from_millis(100),
                            MAX_RETRY_COUNT,
                        );
                        handler.handle_event_with_retry(
                            &contract,
                            (event.clone(), log.clone()),
                            backoff,
                        )
                    });
                    let result = futures::future::join_all(tasks).await;
                    // this event will be marked as handled if at least one handler succeeded.
                    // this because, for the failed events, we already tried to handle them
                    // many times (at this point), and there is no point in trying again.
                    let mark_as_handled = result.iter().any(Result::is_ok);
                    // also, for all the failed event handlers, we should print what went
                    // wrong.
                    result.iter().for_each(|r| {
                        if let Err(e) = r {
                            tracing::error!(?e, %chain_id, "Error while handling the event");
                        }
                    });
                    if mark_as_handled {
                        local_db.set(
                            &format!("LAST_BLOCK_NUMBER_{}", contract.address()),
                            log.block_number.unwrap_or_default().into(),
                        );
                    } else {
                        tracing::error!(%chain_id, "Error while handling event, all handlers failed.");
                        tracing::warn!(%chain_id, "Restarting event watcher ...");
                        // this a transient error, so we will retry again.
                        return Err(backoff::Error::transient(Error::ForceRestart));
                    }
                }

                // move the block pointer to the destination block
                local_db.set(
                    &format!("LAST_BLOCK_NUMBER_{}", contract.address()),
                    dest_block.into(),
                );
                // if we fully synced, we can update the target block number
                let should_cooldown = dest_block == target_block_number;
                if should_cooldown {
                    let duration = Duration::from_secs(10);
                    tracing::trace!("Cooldown a bit for {}ms", duration.as_millis());
                    tokio::time::sleep(duration).await;
                    // update the latest block number
                    target_block_number = provider
                        .get_block_number()
                        .map_err(Into::into)
                        .map_err(backoff::Error::transient)
                        .await?;
                    local_db.set(
                        &format!("TARGET_BLOCK_NUMBER_{}", contract.address()),
                        target_block_number.into(),
                    );
                }
            }
        };
        backoff::future::retry(backoff, task).await?;
        Ok(())
    }
}
