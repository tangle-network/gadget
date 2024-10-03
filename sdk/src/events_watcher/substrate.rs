//! Substrate Event Watcher Module
//!
//! ## Overview
//!
//! Event watcher traits handle the syncing and listening of events for a Substrate network.
//! The event watcher calls into a storage for handling of important state. The run implementation
//! of an event watcher polls for blocks. Implementations of the event watcher trait define an
//! action to take when the specified event is found in a block at the `handle_event` api.

use crate::events_watcher::error::Error;
use std::sync::Arc;
use subxt::OnlineClient;
use subxt_core::utils::AccountId32;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field;
use tangle_subxt::tangle_testnet_runtime::api::services::events::JobCalled;
/// A type alias to extract the event handler type from the event watcher.
pub type EventHandlerFor<RuntimeConfig> = Arc<dyn EventHandler<RuntimeConfig>>;

/// A trait that defines a handler for a specific set of event types.
///
/// The handlers are implemented separately from the watchers, so that we can have
/// one event watcher and many event handlers that will run in parallel.
#[async_trait::async_trait]
pub trait EventHandler<RuntimeConfig>: Send + Sync + 'static
where
    RuntimeConfig: subxt::Config + Send + Sync + 'static,
{
    fn init(&self);

    async fn handle(&self, event: &JobCalled) -> Result<Vec<Field<AccountId32>>, Error>;

    /// Returns the job ID
    fn job_id(&self) -> u8;

    /// Returns the service ID
    fn service_id(&self) -> u64;

    /// Returns the signer
    fn signer(
        &self,
    ) -> &crate::keystore::TanglePairSigner<crate::keystore::sp_core_subxt::sr25519::Pair>;
}

/// Represents a Substrate event watcher.
#[async_trait::async_trait]
pub trait SubstrateEventWatcher<RuntimeConfig>: Send + Sync + 'static
where
    RuntimeConfig: subxt::Config + Send + Sync + 'static,
{
    /// A helper unique tag to help identify the event watcher in the tracing logs.
    const TAG: &'static str;

    /// The name of the pallet that this event watcher is watching.
    const PALLET_NAME: &'static str;

    fn client(&self) -> &OnlineClient<RuntimeConfig>;
    fn handlers(&self) -> &Vec<EventHandlerFor<RuntimeConfig>>;
}
