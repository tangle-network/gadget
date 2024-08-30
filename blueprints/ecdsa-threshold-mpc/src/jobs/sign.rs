use gadget_sdk as sdk;
use std::convert::Infallible;

use color_eyre::{eyre::OptionExt, Result};
use sdk::{
    events_watcher::tangle::TangleEventsWatcher, events_watcher::SubstrateEventWatcher,
    keystore::backend::GenericKeyStore, keystore::Backend, tangle_subxt::*,
};

#[sdk::job(
    id = 1,
    params(msgs, is_prehashed),
    result(_),
    verifier(evm = "HelloBlueprint")
)]
pub fn sign(msgs: Vec<Vec<u8>>, is_prehashed: bool) -> Result<String, Infallible> {
    Ok("Hello World!".to_string())
}
