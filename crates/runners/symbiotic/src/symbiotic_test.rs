use crate::symbiotic::SymbioticConfig;
use gadget_config::protocol::SymbioticContractAddresses;
use gadget_config::{GadgetConfiguration, ProtocolSettings};
use gadget_runner_core::config::BlueprintConfig;
use gadget_runner_core::error::RunnerError;

fn create_test_config() -> GadgetConfiguration {
    let mut config = GadgetConfiguration::default();
    config.http_rpc_endpoint = "http://localhost:8545".to_string();
    config.protocol_settings = ProtocolSettings::Symbiotic(SymbioticContractAddresses {
        operator_registry_address: Default::default(),
        network_registry_address: Default::default(),
        base_delegator_address: Default::default(),
        network_opt_in_service_address: Default::default(),
        vault_opt_in_service_address: Default::default(),
        slasher_address: Default::default(),
        veto_slasher_address: Default::default(),
    });
    config
}

#[tokio::test]
async fn test_requires_registration_invalid_protocol() {
    let config = SymbioticConfig::default();
    let mut env = GadgetConfiguration::default();
    env.protocol_settings = ProtocolSettings::None;

    let result = config.requires_registration(&env).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        RunnerError::InvalidProtocol(_)
    ));
}

// TODO: Run this test with a local testnet
#[tokio::test]
#[ignore]
async fn test_requires_registration_with_mock_node() {
    let config = SymbioticConfig::default();
    let env = create_test_config();

    let result = config.requires_registration(&env).await;
    assert!(result.is_ok());
}
