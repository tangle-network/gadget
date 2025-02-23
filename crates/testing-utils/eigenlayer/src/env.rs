use alloy_primitives::Uint;
use alloy_primitives::{address, Address};
use alloy_provider::Provider;
use eigensdk::utils::middleware::registrycoordinator::IRegistryCoordinator::OperatorSetParam;
use eigensdk::utils::middleware::registrycoordinator::IStakeRegistry::StrategyParams;
use eigensdk::utils::middleware::registrycoordinator::RegistryCoordinator;
use gadget_anvil_testing_utils::get_receipt;
use gadget_config::protocol::EigenlayerContractAddresses;
use gadget_eigenlayer_bindings::pauser_registry::PauserRegistry;
use gadget_logging::info;
use gadget_utils::evm::get_provider_http;

/// The default AVS Directory address on our testnet
pub const AVS_DIRECTORY_ADDR: Address = address!("0000000000000000000000000000000000000000");
/// The default Delegation Manager address on our testnet
pub const DELEGATION_MANAGER_ADDR: Address = address!("dc64a140aa3e981100a9beca4e685f962f0cf6c9");
/// The default ERC20 Mock address on our testnet
pub const ERC20_MOCK_ADDR: Address = address!("7969c5ed335650692bc04293b07f5bf2e7a673c0");
/// The default Mailbox address on our testnet
pub const MAILBOX_ADDR: Address = address!("0000000000000000000000000000000000000000");
/// The default Operator State Retriever address on our testnet
pub const OPERATOR_STATE_RETRIEVER_ADDR: Address =
    address!("1613beb3b2c4f22ee086b2b38c1476a3ce7f78e8");
/// The default Registry Coordinator address on our testnet
pub const REGISTRY_COORDINATOR_ADDR: Address = address!("c3e53f4d16ae77db1c982e75a937b9f60fe63690");
/// The default Service Manager address on our testnet
pub const SERVICE_MANAGER_ADDR: Address = address!("67d269191c92caf3cd7723f116c85e6e9bf55933");
/// The default Strategy Manager address on our testnet
pub const STRATEGY_MANAGER_ADDR: Address = address!("5fc8d32690cc91d4c39d9d3abcbd16989f875707");

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

    let registry_coordinator_address = address!("c3e53f4d16ae77db1c982e75a937b9f60fe63690");
    std::env::set_var(
        "REGISTRY_COORDINATOR_ADDR",
        registry_coordinator_address.to_string(),
    );
    let operator_state_retriever_address = address!("1613beb3b2c4f22ee086b2b38c1476a3ce7f78e8");
    std::env::set_var(
        "OPERATOR_STATE_RETRIEVER_ADDR",
        operator_state_retriever_address.to_string(),
    );
    let delegation_manager_address = address!("dc64a140aa3e981100a9beca4e685f962f0cf6c9");
    std::env::set_var(
        "DELEGATION_MANAGER_ADDR",
        delegation_manager_address.to_string(),
    );
    let service_manager_address = address!("67d269191c92caf3cd7723f116c85e6e9bf55933");
    std::env::set_var("SERVICE_MANAGER_ADDR", service_manager_address.to_string());
    let stake_registry_address = address!("5fc8d32690cc91d4c39d9d3abcbd16989f875707");
    std::env::set_var("STAKE_REGISTRY_ADDR", stake_registry_address.to_string());
    let strategy_manager_address = address!("5fc8d32690cc91d4c39d9d3abcbd16989f875707");
    std::env::set_var(
        "STRATEGY_MANAGER_ADDR",
        strategy_manager_address.to_string(),
    );
    let erc20_mock_address = address!("7969c5ed335650692bc04293b07f5bf2e7a673c0");
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
    let _receipt = get_receipt(registry_coordinator.createQuorum(
        operator_set_params,
        Uint::from(0),
        vec![strategy_params],
    ))
    .await
    .unwrap();

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
