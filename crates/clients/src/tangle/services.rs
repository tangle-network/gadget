use crate::error::Error;
use sp_core::Encode;
use subxt::backend::BlockRef;
use subxt::utils::AccountId32;
use subxt::utils::H256;
use subxt::{Config, OnlineClient};
use tangle_subxt::tangle_testnet_runtime::api;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::sp_runtime::DispatchError;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::ServiceBlueprint;

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

    /// Get the associated RPC client
    pub fn rpc_client(&self) -> &OnlineClient<C> {
        &self.rpc_client
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
            .map_err(|e| Error::Client(e.to_string()))?
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
            .map_err(|e| Error::Client(e.to_string()))?
            .map_err(|err| self.dispatch_error_to_sdk_error(err, &at_block))?;

        Ok(ret)
    }

    pub fn dispatch_error_to_sdk_error(&self, err: DispatchError, at: &[u8; 32]) -> Error {
        let metadata = self.rpc_client.metadata();
        let at_hex = hex::encode(at);
        let dispatch_error =
            tangle_subxt::subxt::error::DispatchError::decode_from(err.encode(), metadata);
        match dispatch_error {
            Ok(dispatch_error) => Error::Other(format!("{dispatch_error}")),
            Err(err) => Error::Other(format!(
                "Failed to construct DispatchError at block 0x{at_hex}: {err}"
            )),
        }
    }
}
