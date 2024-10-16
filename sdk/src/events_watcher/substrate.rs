//! Substrate Event Watcher Module
//!
//! ## Overview
//!
//! Event watcher traits handle the syncing and listening of events for a Substrate network.
//! The event watcher calls into a storage for handling of important state. The run implementation
//! of an event watcher polls for blocks. Implementations of the event watcher trait define an
//! action to take when the specified event is found in a block at the `handle_event` api.
use std::sync::Arc;
use subxt_core::utils::AccountId32;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field;

/// A type alias to extract the event handler type from the event watcher.
pub type EventHandlerFor<RuntimeConfig, Evt> = Arc<dyn EventHandler<RuntimeConfig, Evt>>;

/// A trait that defines a handler for a specific set of event types.
///
/// The handlers are implemented separately from the watchers, so that we can have
/// one event watcher and many event handlers that will run in parallel.
#[async_trait::async_trait]
pub trait EventHandler<RuntimeConfig, Evt>: Send + Sync + 'static
where
    RuntimeConfig: subxt::Config + Send + Sync + 'static,
    Evt: Send + Sync + 'static,
{
    async fn handle(
        &self,
        event: &Evt,
    ) -> Result<Vec<Field<AccountId32>>, crate::events_watcher::Error>;

    /// Returns the job ID
    fn job_id(&self) -> u8;

    /// Returns the service ID
    fn service_id(&self) -> u64;

    /// Returns the signer
    fn signer(
        &self,
    ) -> &crate::keystore::TanglePairSigner<crate::keystore::sp_core_subxt::sr25519::Pair>;
}
