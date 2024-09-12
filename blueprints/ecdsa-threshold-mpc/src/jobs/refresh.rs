use gadget_sdk as sdk;
use std::convert::Infallible;

use color_eyre::Result;

#[sdk::job(
    id = 2,
    params(keygen_id, new_parties, t),
    result(_),
    verifier(evm = "HelloBlueprint")
)]
pub fn refresh(keygen_id: u32, new_parties: Vec<u8>, t: u16) -> Result<String, Infallible> {
    Ok("Hello World!".to_string())
}
