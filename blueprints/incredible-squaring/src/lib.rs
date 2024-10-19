use gadget_sdk::clients::tangle::runtime::TangleClient;
use gadget_sdk::event_listener::tangle::{TangleJobEvent, TangleEventListener};
use gadget_sdk::job;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::pallet_services::module::Event;

/// Returns x^2 saturating to [`u64::MAX`] if overflow occurs.
#[job(
    id = 0,
    params(x),
    result(_),
    event_listener(
        listener = TangleEventListener<Event, TangleClient>,
        event = Event, // Without specifying a pre-processor, it will default to TangleJobEvent
    ),
    verifier(evm = "IncredibleSquaringBlueprint")
)]
pub fn xsquare(
    x: TangleJobEvent<TangleClient>,
    context: TangleClient,
) -> Result<u64, gadget_sdk::Error> {
    Ok(1)
}