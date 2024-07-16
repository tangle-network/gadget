/// An event watcher for the Tangle network.
#[derive(Debug, Default, Clone, Copy)]
pub struct TangleEventsWatcher;

/// A Type alias for the Tangle configuration [`subxt::SubstrateConfig`].
pub type TangleConfig = subxt::PolkadotConfig;

#[async_trait::async_trait]
impl super::SubstrateEventWatcher<TangleConfig> for TangleEventsWatcher {
    const TAG: &'static str = "tangle";
    const PALLET_NAME: &'static str = "Services";
}
