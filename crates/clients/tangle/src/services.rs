use crate::error::Error;
use crate::error::{Result, TangleDispatchError};
use gadget_std::string::ToString;
use gadget_std::vec::Vec;
use subxt::backend::BlockRef;
use subxt::utils::AccountId32;
use subxt::utils::H256;
use subxt::{Config, OnlineClient};
use tangle_subxt::subxt;
use tangle_subxt::tangle_testnet_runtime::api;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::sp_arithmetic::per_things::Percent;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::{
    self, ServiceBlueprint,
};

/// A client for interacting with the services API
#[derive(Debug, Clone)]
pub struct TangleServicesClient<C: Config> {
    pub rpc_client: OnlineClient<C>,
}

impl<C: Config> TangleServicesClient<C> {
    /// Create a new services client
    pub fn new(rpc_client: OnlineClient<C>) -> Self {
        Self { rpc_client }
    }
}

impl<C: Config> gadget_std::ops::Deref for TangleServicesClient<C> {
    type Target = OnlineClient<C>;

    fn deref(&self) -> &Self::Target {
        &self.rpc_client
    }
}

/// A list of services provided by an operator, along with their blueprint
pub type RpcServicesWithBlueprint = services::RpcServicesWithBlueprint<AccountId32, u64, u128>;

impl<C: Config> TangleServicesClient<C>
where
    BlockRef<<C as Config>::Hash>: From<BlockRef<H256>>,
{
    /// Get the Blueprint with the given ID at the given block
    ///
    /// # Errors
    ///
    /// Returns an error if the Blueprint could not be fetched
    pub async fn get_blueprint_by_id(
        &self,
        at: [u8; 32],
        blueprint_id: u64,
    ) -> Result<Option<ServiceBlueprint>> {
        let call = api::storage().services().blueprints(blueprint_id);
        let at = BlockRef::from_hash(H256::from_slice(&at));
        let ret = self.rpc_client.storage().at(at).fetch(&call).await?;
        match ret {
            Some(blueprints) => Ok(Some(blueprints.1)),
            None => Ok(None),
        }
    }

    /// Get the Blueprints provided by the operator at `address`
    ///
    /// # Errors
    ///
    /// Returns an error if the Blueprints could not be fetched
    pub async fn query_operator_blueprints(
        &self,
        at_block: [u8; 32],
        address: AccountId32,
    ) -> Result<Vec<RpcServicesWithBlueprint>> {
        let call = api::apis()
            .services_api()
            .query_services_with_blueprints_by_operator(address);
        let at = BlockRef::from_hash(H256::from_slice(&at_block));
        let ret = self
            .rpc_client
            .runtime_api()
            .at(at)
            .call(call)
            .await?
            .map_err(TangleDispatchError)?;

        Ok(ret)
    }

    /// Query the current Blueprint owner
    pub async fn current_blueprint_owner(
        &self,
        at: [u8; 32],
        blueprint_id: u64,
    ) -> Result<AccountId32> {
        let call = api::storage().services().blueprints(blueprint_id);
        let at = BlockRef::from_hash(H256::from_slice(&at));
        let ret = self.rpc_client.storage().at(at).fetch(&call).await?;
        match ret {
            Some(blueprints) => Ok(blueprints.0),
            None => Err(Error::Other("Blueprint not found".to_string())),
        }
    }

    /// Get the current service operators with their restake exposure
    pub async fn current_service_operators(
        &self,
        at: [u8; 32],
        service_id: u64,
    ) -> Result<Vec<(AccountId32, Percent)>> {
        let call = api::storage().services().instances(service_id);
        let at = BlockRef::from_hash(H256::from_slice(&at));
        let ret = self.rpc_client.storage().at(at).fetch(&call).await?;
        match ret {
            Some(instances) => Ok(instances.operators.0),
            None => Ok(Vec::new()),
        }
    }
}
