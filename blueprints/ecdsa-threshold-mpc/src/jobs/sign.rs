use gadget_sdk as sdk;
use std::convert::Infallible;

use color_eyre::Result;

#[sdk::job(
    id = 1,
    params(msgs, is_prehashed),
    result(_),
    verifier(evm = "HelloBlueprint")
)]
pub fn sign(msgs: Vec<u8>, is_prehashed: bool) -> Result<String, Infallible> {
    Ok("Hello World!".to_string())
}
