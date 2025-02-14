use blueprint_sdk::config::GadgetConfiguration;
use blueprint_sdk::contexts::keystore::KeystoreContext;
use blueprint_sdk::crypto::sp_core::SpSr25519;
use blueprint_sdk::event_listeners::core::InitializableEventHandler;
use blueprint_sdk::event_listeners::tangle::events::{TangleEvent, TangleEventListener};
use blueprint_sdk::job;
use blueprint_sdk::keystore::backends::Backend;
use blueprint_sdk::logging::info;
use blueprint_sdk::macros::contexts::{ServicesContext, TangleClientContext};
use blueprint_sdk::tangle_subxt::tangle_testnet_runtime::api;

#[derive(Clone, TangleClientContext, ServicesContext)]
pub struct MyContext {
    #[config]
    sdk_config: GadgetConfiguration,
    #[call_id]
    call_id: Option<u64>,
}

pub async fn constructor(
    env: GadgetConfiguration,
) -> color_eyre::Result<impl InitializableEventHandler + Clone + Send + Sync + 'static> {
    let signer = env
        .clone()
        .keystore()
        .first_local::<SpSr25519>()
        .map_err(|e| color_eyre::eyre::eyre!(e))?;

    info!("Starting the event watcher for {:?} ...", signer.0);
    RawEventHandler::new(
        &env,
        MyContext {
            sdk_config: env.clone(),
            call_id: None,
        },
    )
    .await
    .map_err(|e| color_eyre::eyre::eyre!(e))
}

#[job(
    id = 2,
    event_listener(
        listener = TangleEventListener<MyContext>,
    ),
)]
pub fn raw(event: TangleEvent<MyContext>, context: MyContext) -> Result<u64, blueprint_sdk::Error> {
    if let Some(balance_transfer) = event
        .evt
        .as_event::<api::balances::events::Transfer>()
        .ok()
        .flatten()
    {
        info!("Found a balance transfer: {balance_transfer:?}");

        let result = std::env::var("RAW_EVENT_RESULT").unwrap_or("0".to_string());
        let result = result.parse::<u64>().unwrap_or(0);
        let result = result + 1;
        std::env::set_var("RAW_EVENT_RESULT", result.to_string());

        return Ok(1);
    }
    Ok(0)
}
