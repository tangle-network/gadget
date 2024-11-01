use gadget_sdk::event_listener::tangle::{TangleEvent, TangleEventListener};
use gadget_sdk::job;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api;

#[derive(Clone)]
pub struct MyContext;

#[job(
    id = 0,
    event_listener(
        listener = TangleEventListener<MyContext>,
    ),
)]
pub fn raw(event: TangleEvent<MyContext>, context: MyContext) -> Result<u64, gadget_sdk::Error> {
    if let Some(balance_transfer) = event
        .evt
        .as_event::<api::balances::events::Transfer>()
        .ok()
        .flatten()
    {
        gadget_sdk::info!("Found a balance transfer: {balance_transfer:?}");
    }
    Ok(0)
}
