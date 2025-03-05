use super::*;
use alloy_primitives::address;
use alloy_provider::Provider;
use client::EigenlayerClient;
use gadget_anvil_testing_utils::{start_default_anvil_testnet, Container};
use gadget_config::{
    load, protocol::EigenlayerContractAddresses, supported_chains::SupportedChains, ContextConfig,
    GadgetConfiguration,
};

struct TestEnvironment {
    // Unused, stored here to keep it from dropping early
    _container: Container,
    #[expect(dead_code)]
    http_endpoint: String,
    #[expect(dead_code)]
    ws_endpoint: String,
    config: GadgetConfiguration,
}

async fn setup_test_environment() -> TestEnvironment {
    let (_container, http_endpoint, ws_endpoint) = start_default_anvil_testnet(false).await;

    // Create test configuration
    let context_config = ContextConfig::create_eigenlayer_config(
        http_endpoint.parse().unwrap(),
        ws_endpoint.parse().unwrap(),
        String::new(),
        None,
        SupportedChains::LocalTestnet,
        EigenlayerContractAddresses::default(),
    );
    let config = load(context_config).unwrap();

    TestEnvironment {
        _container,
        http_endpoint,
        ws_endpoint,
        config,
    }
}

#[tokio::test]
async fn get_provider_http() {
    let env = setup_test_environment().await;
    let client = EigenlayerClient::new(env.config.clone());
    let provider = client.get_provider_http();
    assert!(provider.get_block_number().await.is_ok());
}

#[tokio::test]
async fn get_provider_ws() {
    let env = setup_test_environment().await;
    let client = EigenlayerClient::new(env.config.clone());
    let provider = client.get_provider_ws().await.unwrap();
    assert!(provider.get_block_number().await.is_ok());
}

#[tokio::test]
async fn get_allocation_manager_address() {
    let env = setup_test_environment().await;
    let client = EigenlayerClient::new(env.config.clone());
    let delegation_manager_addr = address!("dc64a140aa3e981100a9beca4e685f962f0cf6c9");
    let result = client
        .get_allocation_manager_address(delegation_manager_addr)
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn avs_registry_reader() {
    let env = setup_test_environment().await;
    let client = EigenlayerClient::new(env.config.clone());
    let result = client.avs_registry_reader().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn avs_registry_writer() {
    let env = setup_test_environment().await;
    let client = EigenlayerClient::new(env.config.clone());
    let private_key = "0000000000000000000000000000000000000000000000000000000000000001";
    let result = client.avs_registry_writer(private_key.to_string()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn operator_info_service() {
    let env = setup_test_environment().await;
    let client = EigenlayerClient::new(env.config.clone());
    let result = client.operator_info_service_in_memory().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn get_operator_stake_in_quorums() {
    let env = setup_test_environment().await;
    let client = EigenlayerClient::new(env.config.clone());
    let result = client
        .get_operator_stake_in_quorums_at_block(200, vec![0].into())
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn get_operator_id() {
    let env = setup_test_environment().await;
    let client = EigenlayerClient::new(env.config.clone());
    let operator_addr = address!("f39fd6e51aad88f6f4ce6ab8827279cfffb92266");
    let result = client.get_operator_id(operator_addr).await;
    assert!(result.is_ok());
}
