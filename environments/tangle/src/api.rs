use gadget_common::config::DebugLogger;
use gadget_common::tangle_runtime::api::runtime_types::tangle_primitives::services;
use gadget_common::tangle_runtime::AccountId32;
use gadget_common::tangle_subxt::subxt::backend::BlockRef;
use gadget_common::tangle_subxt::subxt::utils::H256;
use gadget_common::tangle_subxt::subxt::{Config, OnlineClient};
use gadget_common::tangle_subxt::tangle_testnet_runtime::api;

pub type BlockHash = [u8; 32];

/// A client for interacting with the services API
/// TODO: Make this the object that is used for getting services and submitting transactions as necessary.
pub struct ServicesClient<C: Config> {
    rpc_client: OnlineClient<C>,
    #[allow(dead_code)]
    logger: DebugLogger,
}

impl<C: Config> ServicesClient<C> {
    pub fn new(logger: DebugLogger, rpc_client: OnlineClient<C>) -> Self {
        Self { logger, rpc_client }
    }

    pub fn rpc_client(&self) -> &OnlineClient<C> {
        &self.rpc_client
    }
}

pub type RpcServicesWithBlueprint = services::RpcServicesWithBlueprint<AccountId32, u64>;

impl<C: Config> ServicesClient<C>
where
    BlockRef<<C as Config>::Hash>: From<BlockRef<H256>>,
{
    pub async fn get_blueprint_by_id(
        &self,
        at: [u8; 32],
        blueprint_id: u64,
    ) -> Result<Option<RpcServicesWithBlueprint>, gadget_common::Error> {
        let call = api::storage().services().blueprints(blueprint_id);
        let at = BlockRef::from_hash(H256::from_slice(&at));
        let ret: Option<RpcServicesWithBlueprint> = self
            .rpc_client
            .runtime_api()
            .at(at)
            .call(call)
            .await
            .map_err(|e| gadget_common::Error::ClientError { err: e.to_string() })?
            .map_err(|err| gadget_common::Error::ClientError {
                err: format!("{err:?}"),
            })?;

        Ok(ret)
    }

    pub async fn query_operator_blueprints(
        &self,
        at: [u8; 32],
        address: AccountId32,
    ) -> Result<Vec<RpcServicesWithBlueprint>, gadget_common::Error> {
        let call = api::apis()
            .services_api()
            .query_services_with_blueprints_by_operator(address);
        let at = BlockRef::from_hash(H256::from_slice(&at));
        let ret: Vec<RpcServicesWithBlueprint> = self
            .rpc_client
            .runtime_api()
            .at(at)
            .call(call)
            .await
            .map_err(|e| gadget_common::Error::ClientError { err: e.to_string() })?
            .map_err(|err| gadget_common::Error::ClientError {
                err: format!("{err:?}"),
            })?;

        Ok(ret)
    }
}
