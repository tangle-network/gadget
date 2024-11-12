use gadget_sdk::config::StdGadgetConfiguration;
use gadget_sdk::event_listener::tangle::{TangleEvent, TangleEventListener};
use gadget_sdk::event_utils::InitializableEventHandler;
use gadget_sdk::job;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api;

#[derive(Clone)]
pub struct MyContext;

pub async fn constructor(
    env: StdGadgetConfiguration,
) -> color_eyre::Result<impl InitializableEventHandler> {
    use gadget_sdk::subxt_core::tx::signer::Signer;

    let signer = env
        .first_sr25519_signer()
        .map_err(|e| color_eyre::eyre::eyre!(e))?;

    gadget_sdk::info!("Starting the event watcher for {} ...", signer.account_id());
    RawEventHandler::new(&env, MyContext)
        .await
        .map_err(|e| color_eyre::eyre::eyre!(e))
}

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
