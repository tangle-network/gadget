use alloy_primitives::Uint;
use alloy_primitives::{address, Address};
use alloy_provider::Provider;
use eigensdk::utils::rewardsv2::middleware::registrycoordinator::IRegistryCoordinator::OperatorSetParam;
use eigensdk::utils::rewardsv2::middleware::registrycoordinator::IStakeRegistry::StrategyParams;
use eigensdk::utils::rewardsv2::middleware::registrycoordinator::RegistryCoordinator;
use gadget_anvil_testing_utils::get_receipt;
use gadget_config::protocol::EigenlayerContractAddresses;
use gadget_eigenlayer_bindings::pauser_registry::PauserRegistry;
use gadget_logging::info;
use gadget_utils::evm::get_provider_http;

pub const AVS_DIRECTORY_ADDR: Address = address!("0165878A594ca255338adfa4d48449f69242Eb8F");
pub const DELEGATION_MANAGER_ADDR: Address = address!("Dc64a140Aa3E981100a9becA4E685f962f0cF6C9");
pub const ERC20_MOCK_ADDR: Address = address!("FD471836031dc5108809D173A067e8486B9047A3");
pub const MAILBOX_ADDR: Address = address!("0000000000000000000000000000000000000000");
pub const OPERATOR_STATE_RETRIEVER_ADDR: Address =
    address!("f5059a5D33d5853360D16C683c16e67980206f36");
pub const REGISTRY_COORDINATOR_ADDR: Address = address!("9E545E3C0baAB3E08CdfD552C960A1050f373042");
pub const SERVICE_MANAGER_ADDR: Address = address!("c3e53F4d16Ae77Db1c982e75a937B9f60FE63690");
pub const STRATEGY_MANAGER_ADDR: Address = address!("5FC8d32690cc91D4c39d9d3abcBD16989F875707");

pub struct EigenlayerTestEnvironment {
    pub http_endpoint: String,
    pub ws_endpoint: String,
    pub accounts: Vec<Address>,
    pub eigenlayer_contract_addresses: EigenlayerContractAddresses,
    pub pauser_registry_address: Address,
}

/// Sets up the test environment for the EigenLayer Blueprint.
///
/// # Description
/// - Sets all the necessary environment variables for the necessary EigenLayer Contract Addresses.
/// - Returns a [`EigenlayerTestEnvironment`] struct containing the test environment state.
pub async fn setup_eigenlayer_test_environment(
    http_endpoint: &str,
    ws_endpoint: &str,
) -> EigenlayerTestEnvironment {
    let provider = get_provider_http(http_endpoint);

    let accounts = provider.get_accounts().await.unwrap();

    let registry_coordinator_address = address!("9E545E3C0baAB3E08CdfD552C960A1050f373042");
    std::env::set_var(
        "REGISTRY_COORDINATOR_ADDR",
        registry_coordinator_address.to_string(),
    );
    let operator_state_retriever_address = address!("f5059a5D33d5853360D16C683c16e67980206f36");
    std::env::set_var(
        "OPERATOR_STATE_RETRIEVER_ADDR",
        operator_state_retriever_address.to_string(),
    );
    let delegation_manager_address = address!("Dc64a140Aa3E981100a9becA4E685f962f0cF6C9");
    std::env::set_var(
        "DELEGATION_MANAGER_ADDR",
        delegation_manager_address.to_string(),
    );
    let service_manager_address = address!("c3e53F4d16Ae77Db1c982e75a937B9f60FE63690");
    std::env::set_var("SERVICE_MANAGER_ADDR", service_manager_address.to_string());
    let stake_registry_address = address!("5fc8d32690cc91d4c39d9d3abcbd16989f875707");
    std::env::set_var("STAKE_REGISTRY_ADDR", stake_registry_address.to_string());
    let strategy_manager_address = address!("5FC8d32690cc91D4c39d9d3abcBD16989F875707");
    std::env::set_var(
        "STRATEGY_MANAGER_ADDR",
        strategy_manager_address.to_string(),
    );
    let erc20_mock_address = address!("FD471836031dc5108809D173A067e8486B9047A3");
    std::env::set_var("ERC20_MOCK_ADDR", erc20_mock_address.to_string());

    let pauser_registry = PauserRegistry::deploy(provider.clone(), accounts.clone(), accounts[0])
        .await
        .unwrap();
    let pauser_registry_address = *pauser_registry.address();

    let registry_coordinator =
        RegistryCoordinator::new(registry_coordinator_address, provider.clone());

    let operator_set_params = OperatorSetParam {
        maxOperatorCount: 10,
        kickBIPsOfOperatorStake: 100,
        kickBIPsOfTotalStake: 1000,
    };
    let strategy_params = StrategyParams {
        strategy: erc20_mock_address,
        multiplier: Uint::from(1),
    };

    info!("Creating Quorum");
    let receipt = get_receipt(registry_coordinator.createQuorum(
        operator_set_params,
        Uint::from(0),
        vec![strategy_params],
    ))
    .await
    .unwrap();

    info!("Quorum created: {:?}", receipt);

    assert!(receipt.status());

    info!("Setup Eigenlayer test environment");

    EigenlayerTestEnvironment {
        http_endpoint: http_endpoint.to_string(),
        ws_endpoint: ws_endpoint.to_string(),
        accounts,
        eigenlayer_contract_addresses: EigenlayerContractAddresses {
            registry_coordinator_address,
            operator_state_retriever_address,
            delegation_manager_address,
            service_manager_address,
            stake_registry_address,
            strategy_manager_address,
            avs_directory_address: Default::default(),
            rewards_coordinator_address: Default::default(),
        },
        pauser_registry_address,
    }
}
