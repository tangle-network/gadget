use gadget_sdk as sdk;
use std::convert::Infallible;

use color_eyre::Result;

#[sdk::job(
    id = 0,
    params(parties, t, num_keys),
    result(_),
    verifier(evm = "HelloBlueprint")
)]
pub fn keygen(parties: Vec<u16>, t: u16, num_keys: u16) -> Result<String, Infallible> {
    Ok("Hello World!".to_string())
}
