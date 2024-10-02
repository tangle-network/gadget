use async_trait::async_trait;
use gadget_sdk::clients::tangle::runtime::TangleConfig;
use gadget_sdk::config::GadgetConfiguration;
use gadget_sdk::event_listener::substrate::{
    SubstrateEventListener, SubstrateEventListenerContext,
};
use gadget_sdk::ext::subxt::blocks::Block;
use gadget_sdk::ext::subxt::OnlineClient;
use gadget_sdk::{job, Error};
use std::convert::Infallible;
//pub mod eigenlayer;

#[derive(Copy, Clone)]
pub struct LocalSubstrateTestnetContext;

#[async_trait]
impl SubstrateEventListenerContext<TangleConfig> for LocalSubstrateTestnetContext {
    async fn handle_event(
        &self,
        event: Block<TangleConfig, OnlineClient<TangleConfig>>,
        _client: &OnlineClient<TangleConfig>,
    ) -> Result<(), Error> {
        gadget_sdk::info!("Received block: {:?}", event.number());
        Ok(())
    }
}

type ExtrinsicT = gadget_sdk::tangle_subxt::subxt::ext::sp_runtime::testing::ExtrinsicWrapper<()>;
type LocalTestnetBlock =
    gadget_sdk::tangle_subxt::subxt::ext::sp_runtime::testing::Block<ExtrinsicT>;

/// Returns x^2 saturating to [`u64::MAX`] if overflow occurs.
#[job(
    id = 0,
    params(x),
    result(_),
    event_listener(SubstrateEventListener::<TangleConfig, LocalTestnetBlock, LocalSubstrateTestnetContext>),
    verifier(evm = "IncredibleSquaringBlueprint")
)]
pub fn xsquare(
    context: LocalSubstrateTestnetContext,
    x: u64,
    env: GadgetConfiguration<parking_lot::RawRwLock>,
) -> Result<u64, Infallible> {
    Ok(x.saturating_pow(2u32))
}
