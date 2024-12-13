use std::io::Error;
use subxt::backend::BlockRef;
use subxt::utils::AccountId32;
use subxt::utils::H256;
use subxt::{Config, OnlineClient};
use tangle_subxt::tangle_testnet_runtime::api;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::sp_arithmetic::per_things::Percent;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::{
    self, ServiceBlueprint,
};

/// A client for interacting with the services API
#[derive(Debug)]
pub struct ServicesClient<C: Config> {
    rpc_client: OnlineClient<C>,
}

impl<C: Config> ServicesClient<C> {
    /// Create a new services client
    pub fn new(rpc_client: OnlineClient<C>) -> Self {
        Self { rpc_client }
    }
}

/// A list of services provided by an operator, along with their blueprint
pub type RpcServicesWithBlueprint = services::RpcServicesWithBlueprint<AccountId32, u64, u128>;

impl<C: Config> ServicesClient<C>
where
    BlockRef<<C as Config>::Hash>: From<BlockRef<H256>>,
{
    /// Get the blueprint with the given ID at the given block
    ///
    /// # Errors
    ///
    /// Returns an error if the blueprint could not be fetched
    pub async fn get_blueprint_by_id(
        &self,
        at: [u8; 32],
        blueprint_id: u64,
    ) -> Result<Option<ServiceBlueprint>, Error> {
        let call = api::storage().services().blueprints(blueprint_id);
        let at = BlockRef::from_hash(H256::from_slice(&at));
        let ret: Option<ServiceBlueprint> = self
            .rpc_client
            .storage()
            .at(at)
            .fetch(&call)
            .await
            .map_err(|e| Error::new(std::io::ErrorKind::Other, e))?
            .map(|r| r.1);

        Ok(ret)
    }

    /// Get the services provided by the operator at `address`
    ///
    /// # Errors
    ///
    /// Returns an error if the services could not be fetched
    pub async fn query_operator_blueprints(
        &self,
        at_block: [u8; 32],
        address: AccountId32,
    ) -> Result<Vec<RpcServicesWithBlueprint>, Error> {
        let call = api::apis()
            .services_api()
            .query_services_with_blueprints_by_operator(address);
        let at = BlockRef::from_hash(H256::from_slice(&at_block));
        let ret: Vec<RpcServicesWithBlueprint> = self
            .rpc_client
            .runtime_api()
            .at(at)
            .call(call)
            .await
            .map_err(|e| Error::new(std::io::ErrorKind::Other, e))?
            .map_err(|err| self.dispatch_error_to_sdk_error(err, &at_block))?;

        Ok(ret)
    }

    /// Get the current blueprint information
    pub async fn current_blueprint(&self, at: [u8; 32]) -> Result<ServiceBlueprint, subxt::Error> {
        let call = api::storage().services().blueprints(0);
        let at = BlockRef::from_hash(H256::from_slice(&at));
        let ret = self
            .rpc_client
            .storage()
            .at(at)
            .fetch(&call)
            .await?
            .ok_or_else(|| Error::new(std::io::ErrorKind::Other, "Blueprint not found"))?;
        Ok(ret)
    }

    /// Query the current blueprint owner
    pub async fn current_blueprint_owner(&self, at: [u8; 32]) -> Result<AccountId32, subxt::Error> {
        let call = api::storage().services().blueprint_owners(0);
        let at = BlockRef::from_hash(H256::from_slice(&at));
        let ret = self
            .rpc_client
            .storage()
            .at(at)
            .fetch(&call)
            .await?
            .ok_or_else(|| Error::new(std::io::ErrorKind::Other, "Blueprint owner not found"))?;
        Ok(ret)
    }

    /// Get the current service operators with their restake exposure
    pub async fn current_service_operators(
        &self,
        at: [u8; 32],
    ) -> Result<Vec<(AccountId32, Percent)>, subxt::Error> {
        let call = api::storage().services().service_operators(0);
        let at = BlockRef::from_hash(H256::from_slice(&at));
        let ret = self
            .rpc_client
            .storage()
            .at(at)
            .fetch(&call)
            .await?
            .ok_or_else(|| subxt::Error::Other("Service operators not found".into()))?;
        Ok(ret)
    }
}
