use gadget_sdk::clients::tangle::runtime::TangleClient;
use gadget_sdk::event_listener::tangle::{TangleEventListener, TangleJobEvent, TangleJobResult};
use gadget_sdk::job;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::pallet_services::module::Event;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field;
use gadget_sdk::event_listener::tangle::{services_pre_processor, services_post_processor};

/// Returns x^2 saturating to [`u64::MAX`] if overflow occurs.
#[job(
    id = 0,
    params(x),
    result(_),
    event_listener(
        listener = TangleEventListener<Event, TangleClient>,
        event = Event,
        pre_processor = services_pre_processor,
        post_processor = services_post_processor,
    ),
    verifier(evm = "IncredibleSquaringBlueprint")
)]
pub fn xsquare(
    x: u64,
    context: TangleClient,
) -> Result<u64, gadget_sdk::Error> {
    Ok(x.saturating_pow(2))
}
