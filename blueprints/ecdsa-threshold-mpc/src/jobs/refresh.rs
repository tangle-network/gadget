use gadget_sdk as sdk;
use std::convert::Infallible;

use color_eyre::{eyre::OptionExt, Result};
use sdk::{
    events_watcher::tangle::TangleEventsWatcher, events_watcher::SubstrateEventWatcher,
    keystore::backend::GenericKeyStore, keystore::Backend, tangle_subxt::*,
};

#[sdk::job(
    id = 2,
    params(keygen_id, new_parties, t),
    result(_),
    verifier(evm = "HelloBlueprint")
)]
pub fn refresh(keygen_id: u32, new_parties: Vec<u16>, t: u16) -> Result<String, Infallible> {
    Ok("Hello World!".to_string())
}
