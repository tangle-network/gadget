pub use crate::helpers::get_receipt;
pub use alloy_primitives::{address, Address, Bytes, U256};
pub use alloy_provider::Provider;
pub use gadget_sdk::config::protocol::EigenlayerContractAddresses;
pub use gadget_sdk::futures::StreamExt;
pub use gadget_sdk::tokio::sync::Mutex;
pub use gadget_sdk::utils::evm::{get_provider_http, get_provider_ws};
pub use gadget_sdk::{error, info};
pub use std::sync::Arc;
pub use std::time::Duration;

pub const AVS_DIRECTORY_ADDR: Address = address!("0000000000000000000000000000000000000000");
pub const DELEGATION_MANAGER_ADDR: Address = address!("dc64a140aa3e981100a9beca4e685f962f0cf6c9");
pub const ERC20_MOCK_ADDR: Address = address!("7969c5ed335650692bc04293b07f5bf2e7a673c0");
pub const MAILBOX_ADDR: Address = address!("0000000000000000000000000000000000000000");
pub const OPERATOR_STATE_RETRIEVER_ADDR: Address =
    address!("1613beb3b2c4f22ee086b2b38c1476a3ce7f78e8");
pub const REGISTRY_COORDINATOR_ADDR: Address = address!("c3e53f4d16ae77db1c982e75a937b9f60fe63690");
pub const SERVICE_MANAGER_ADDR: Address = address!("67d269191c92caf3cd7723f116c85e6e9bf55933");
pub const STRATEGY_MANAGER_ADDR: Address = address!("5fc8d32690cc91d4c39d9d3abcbd16989f875707");

pub struct EigenlayerTestEnvironment {
    pub http_endpoint: String,
    pub ws_endpoint: String,
    pub accounts: Vec<Address>,
    pub eigenlayer_contract_addresses: EigenlayerContractAddresses,
    pub pauser_registry_address: Address,
}
