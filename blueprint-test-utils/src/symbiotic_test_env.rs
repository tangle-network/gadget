use alloy_primitives::Address;
use alloy_provider::Provider;
use gadget_sdk::info;
use gadget_sdk::{
    config::protocol::SymbioticContractAddresses, event_utils::evm::get_provider_http,
};

pub struct SymbioticTestEnvironment {
    pub http_endpoint: String,
    pub ws_endpoint: String,
    pub accounts: Vec<Address>,
    pub symbiotic_contract_addresses: SymbioticContractAddresses,
}

pub async fn setup_symbiotic_test_environment(
    http_endpoint: &str,
    ws_endpoint: &str,
) -> SymbioticTestEnvironment {
    let provider = get_provider_http(http_endpoint);

    let accounts = provider.get_accounts().await.unwrap();
    info!("Setup Symbiotic test environment");

    SymbioticTestEnvironment {
        http_endpoint: http_endpoint.to_string(),
        ws_endpoint: ws_endpoint.to_string(),
        accounts,
        symbiotic_contract_addresses: Default::default(),
    }
}
