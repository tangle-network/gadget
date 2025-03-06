use alloy_primitives::Uint;
use alloy_primitives::{address, Address};
use alloy_provider::Provider;
use eigensdk::utils::rewardsv2::middleware::ecdsastakeregistry::ECDSAStakeRegistry::Quorum;
use eigensdk::utils::slashing::middleware::registrycoordinator::ISlashingRegistryCoordinatorTypes::OperatorSetParam;
use eigensdk::utils::slashing::middleware::registrycoordinator::IStakeRegistryTypes::StrategyParams;
use eigensdk::utils::slashing::middleware::registrycoordinator::RegistryCoordinator;
use gadget_anvil_testing_utils::get_receipt;
use gadget_config::protocol::EigenlayerContractAddresses;
use gadget_logging::{error, info};
use gadget_utils::evm::get_provider_http;

/// The default Allocation Manager address on our testnet
pub const ALLOCATION_MANAGER_ADDR: Address = address!("8A791620dd6260079BF849Dc5567aDC3F2FDC318");
/// The default AVS Directory address on our testnet
pub const AVS_DIRECTORY_ADDR: Address = address!("B7f8BC63BbcaD18155201308C8f3540b07f84F5e");
/// The default Delegation Manager address on our testnet
pub const DELEGATION_MANAGER_ADDR: Address = address!("Cf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9");
/// The default ERC20 Mock address on our testnet
pub const ERC20_MOCK_ADDR: Address = address!("eC4cFde48EAdca2bC63E94BB437BbeAcE1371bF3");
/// The default Operator State Retriever address on our testnet
pub const OPERATOR_STATE_RETRIEVER_ADDR: Address =
    address!("1429859428c0abc9c2c47c8ee9fbaf82cfa0f20f");
/// The default Permission Controller address on our testnet
pub const PERMISSION_CONTROLLER_ADDR: Address =
    address!("322813Fd9A801c5507c9de605d63CEA4f2CE6c44");
/// The default Registry Coordinator address on our testnet
pub const REGISTRY_COORDINATOR_ADDR: Address = address!("4c4a2f8c81640e47606d3fd77b353e87ba015584");
/// The default Service Manager address on our testnet
pub const SERVICE_MANAGER_ADDR: Address = address!("67d269191c92caf3cd7723f116c85e6e9bf55933");
/// The default Strategy Manager address on our testnet
pub const STRATEGY_MANAGER_ADDR: Address = address!("a513E6E4b8f2a923D98304ec87F64353C4D5C853");
pub const STAKE_REGISTRY_ADDR: Address = address!("922d6956c99e12dfeb3224dea977d0939758a1fe");

pub struct EigenlayerTestEnvironment {
    pub http_endpoint: String,
    pub ws_endpoint: String,
    pub accounts: Vec<Address>,
    pub eigenlayer_contract_addresses: EigenlayerContractAddresses,
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

    std::env::set_var(
        "ALLOCATION_MANAGER_ADDR",
        ALLOCATION_MANAGER_ADDR.to_string(),
    );
    std::env::set_var(
        "REGISTRY_COORDINATOR_ADDR",
        REGISTRY_COORDINATOR_ADDR.to_string(),
    );
    std::env::set_var(
        "OPERATOR_STATE_RETRIEVER_ADDR",
        OPERATOR_STATE_RETRIEVER_ADDR.to_string(),
    );
    std::env::set_var(
        "DELEGATION_MANAGER_ADDR",
        DELEGATION_MANAGER_ADDR.to_string(),
    );
    std::env::set_var(
        "PERMISSION_CONTROLLER_ADDR",
        PERMISSION_CONTROLLER_ADDR.to_string(),
    );
    std::env::set_var("SERVICE_MANAGER_ADDR", SERVICE_MANAGER_ADDR.to_string());
    std::env::set_var("STAKE_REGISTRY_ADDR",STAKE_REGISTRY_ADDR.to_string());
    std::env::set_var(
        "STRATEGY_MANAGER_ADDR",
        STRATEGY_MANAGER_ADDR.to_string(),
    );
    std::env::set_var("ERC20_MOCK_ADDR", ERC20_MOCK_ADDR.to_string());

    let registry_coordinator =
        RegistryCoordinator::new(REGISTRY_COORDINATOR_ADDR, provider.clone());

    let operator_set_params = OperatorSetParam {
        maxOperatorCount: 10,
        kickBIPsOfOperatorStake: 100,
        kickBIPsOfTotalStake: 1000,
    };
    let strategy_params = StrategyParams {
        strategy: ERC20_MOCK_ADDR,
        multiplier: Uint::from(1),
    };

    info!("Creating Quorum...");
    let receipt = get_receipt(registry_coordinator.createTotalDelegatedStakeQuorum(
        operator_set_params,
        Uint::from(0),
        vec![strategy_params],
    ))
    .await
    .unwrap();

    dbg!(receipt);

    info!("Setup Eigenlayer test environment");

    EigenlayerTestEnvironment {
        http_endpoint: http_endpoint.to_string(),
        ws_endpoint: ws_endpoint.to_string(),
        accounts,
        eigenlayer_contract_addresses: EigenlayerContractAddresses {
            allocation_manager_address,
            registry_coordinator_address,
            operator_state_retriever_address,
            delegation_manager_address,
            service_manager_address,
            stake_registry_address,
            strategy_manager_address,
            avs_directory_address: Default::default(),
            rewards_coordinator_address: Default::default(),
            permission_controller_address,
        },
    }
}
