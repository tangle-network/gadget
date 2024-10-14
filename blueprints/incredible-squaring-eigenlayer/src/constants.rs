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
    pub static ref SR_SECRET_BYTES: Vec<u8> = env::var("SR_SECRET_BYTES")
        .map(|v| v.into_bytes())
        .unwrap_or_else(|_| vec![0; 32]);
    pub static ref REGISTRY_COORDINATOR_ADDRESS: Address =
        address!("c3e53f4d16ae77db1c982e75a937b9f60fe63690");
    pub static ref OPERATOR_STATE_RETRIEVER_ADDRESS: Address =
        address!("1613beb3b2c4f22ee086b2b38c1476a3ce7f78e8");
    pub static ref DELEGATION_MANAGER_ADDRESS: Address =
        address!("dc64a140aa3e981100a9beca4e685f962f0cf6c9");
    pub static ref STRATEGY_MANAGER_ADDRESS: Address =
        address!("5fc8d32690cc91d4c39d9d3abcbd16989f875707");
    pub static ref AVS_DIRECTORY_ADDRESS: Address =
        address!("0000000000000000000000000000000000000000");
    pub static ref TASK_MANAGER_ADDRESS: Address = env::var("TASK_MANAGER_ADDRESS")
        .map(|addr| addr.parse().expect("Invalid TASK_MANAGER_ADDRESS"))
        .unwrap_or_else(|_| address!("0000000000000000000000000000000000000000"));
    pub static ref OPERATOR_ADDRESS: Address = address!("f39fd6e51aad88f6f4ce6ab8827279cfffb92267");
    pub static ref OPERATOR_METADATA_URL: String =
        "https://github.com/tangle-network/gadget".to_string();
    pub static ref MNEMONIC_SEED: String = env::var("MNEMONIC_SEED").unwrap_or_else(|_| {
        "test test test test test test test test test test test junk".to_string()
    });
}
