use async_trait::async_trait;
use gadget_sdk::clients::tangle::runtime::TangleClient;
use gadget_sdk::event_listener::EventListener;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::balances;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::balances::events::Transfer;
use gadget_sdk::{job, Error};
use std::convert::Infallible;

#[derive(Clone)]
pub struct MyContext {
    pub client: TangleClient,
}

pub struct MyBalanceTransferListener {
    client: TangleClient,
}

#[async_trait]
impl EventListener<Vec<balances::events::Transfer>, MyContext> for MyBalanceTransferListener {
    async fn new(context: &MyContext) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Ok(Self {
            client: context.client.clone(),
        })
    }

    async fn next_event(&mut self) -> Option<Vec<Transfer>> {
        loop {
            let events = self.client.events().at_latest().await.ok()?;
            // TODO edit filter function
            let transfers = events
                .find::<Transfer>()
                .flatten()
                .filter(|evt| true)
                .collect::<Vec<_>>();
            if !transfers.is_empty() {
                return Some(transfers);
            }
        }
    }

    async fn handle_event(&mut self, event: Vec<Transfer>) -> Result<(), Error> {
        todo!()
    }
}

/// Returns x^2 saturating to [`u64::MAX`] if overflow occurs.
#[job(
    id = 0,
    params(x),
    result(_),
    event_listener(TangleEventListener, MyBalanceTransferListener),
    verifier(evm = "IncredibleSquaringBlueprint")
)]
pub fn xsquare(x: u64, context: MyContext) -> Result<u64, Infallible> {
    Ok(x.saturating_pow(2u32))
}
