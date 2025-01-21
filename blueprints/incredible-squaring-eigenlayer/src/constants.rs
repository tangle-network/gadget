use alloy_primitives::{address, Address, U256};
use lazy_static::lazy_static;
use std::env;

// Environment variables with default values
lazy_static! {
    pub static ref SIGNATURE_EXPIRY: U256 = U256::from(86400);
    pub static ref EIGENLAYER_HTTP_ENDPOINT: String = env::var("EIGENLAYER_HTTP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:8545".to_string());
    pub static ref EIGENLAYER_WS_ENDPOINT: String =
        env::var("EIGENLAYER_WS_ENDPOINT").unwrap_or_else(|_| "ws://localhost:8546".to_string());
    pub static ref PRIVATE_KEY: String = env::var("PRIVATE_KEY").unwrap_or_else(|_| {
        "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string()
    });
    pub static ref AGGREGATOR_PRIVATE_KEY: String = env::var("PRIVATE_KEY").unwrap_or_else(|_| {
        "2a871d0798f97d79848a013d4936a73bf4cc922c825d33c1cf7073dff6d409c6".to_string()
    });
    pub static ref TASK_MANAGER_ADDRESS: Address = env::var("TASK_MANAGER_ADDRESS")
        .map(|addr| addr.parse().expect("Invalid TASK_MANAGER_ADDRESS"))
        .unwrap_or_else(|_| address!("0000000000000000000000000000000000000000"));
}

pub const OPERATOR_ADDRESS: Address = address!("f39fd6e51aad88f6f4ce6ab8827279cfffb92266");
pub const OPERATOR_METADATA_URL: &str = "https://github.com/tangle-network/gadget";
