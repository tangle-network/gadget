pub mod reader;
pub mod subscriber;
pub mod writer;

use std::f64::consts::E;

use alloy_primitives::Address;
use alloy_provider::{network::Ethereum, Provider, ProviderBuilder};
use alloy_sol_types::sol;
use alloy_transport::Transport;
use eigen_contracts::RegistryCoordinator;
use eigen_utils::types::AvsError;

sol!(
    #[allow(missing_docs)]
    #[derive(Debug)]
    #[sol(rpc)]
    IncredibleSquaringTaskManager,
    "contracts/out/IncredibleSquaringTaskManager.sol/IncredibleSquaringTaskManager.json"
);

sol!(
    #[allow(missing_docs)]
    #[derive(Debug)]
    #[sol(rpc)]
    IncredibleSquaringServiceManager,
    "contracts/out/IncredibleSquaringServiceManager.sol/IncredibleSquaringServiceManager.json"
);

sol!(
    #[allow(missing_docs)]
    #[derive(Debug)]
    #[sol(rpc)]
    Erc20Mock,
    "contracts/out/Erc20Mock.sol/Erc20Mock.json"
);

#[derive(Debug, Clone, PartialEq)]
pub enum SignerType {
    Mnemonic(String),
    PrivateKey(String),
    Path(String),
    Yubikey(String),
    Trezor(String),
    Ledger(String),
    AwsKms(String),
}
pub struct SetupConfig<T, P>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
{
    pub registry_coordinator_addr: Address,
    pub operator_state_retriever_addr: Address,
    pub eth_client: P,
    pub signer_type: SignerType,
    phantom: std::marker::PhantomData<T>,
}

pub struct AvsManagersBindings<T, P>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
{
    pub task_manager: IncredibleSquaringTaskManager::IncredibleSquaringTaskManagerInstance<T, P>,
    pub service_manager:
        IncredibleSquaringServiceManager::IncredibleSquaringServiceManagerInstance<T, P>,
    eth_client: P,
}

impl<T, P> AvsManagersBindings<T, P>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
{
    pub async fn new(
        registry_coordinator_addr: Address,
        operator_state_retriever_addr: Address,
        eth_client: P,
    ) -> Result<Self, AvsError> {
        let registry_coordinator = RegistryCoordinator::RegistryCoordinatorInstance::new(
            registry_coordinator_addr,
            eth_client.clone(),
        );
        let service_manager_addr = registry_coordinator
            .serviceManager()
            .call()
            .await
            .map(|x| x._0)?;
        let service_manager =
            IncredibleSquaringServiceManager::IncredibleSquaringServiceManagerInstance::new(
                service_manager_addr,
                eth_client.clone(),
            );

        let task_manager_addr = service_manager
            .incredibleSquaringTaskManager()
            .call()
            .await
            .map(|x| x._0)?;
        let task_manager =
            IncredibleSquaringTaskManager::IncredibleSquaringTaskManagerInstance::new(
                task_manager_addr,
                eth_client.clone(),
            );

        Ok(Self {
            task_manager,
            service_manager,
            eth_client,
        })
    }

    pub async fn get_erc20_mock(
        &self,
        token_addr: Address,
    ) -> Result<Erc20Mock::Erc20MockInstance<T, P>, AvsError> {
        let erc20_mock = Erc20Mock::new(token_addr, self.eth_client.clone());
        Ok(erc20_mock)
    }
}
