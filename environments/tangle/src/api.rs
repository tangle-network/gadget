use gadget_common::config::DebugLogger;
use gadget_common::tangle_runtime::api::runtime_apis::services_api::types::QueryServicesWithBlueprintsByOperator;
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
    pub async fn query_restaker_blueprints(
        &self,
        at: [u8; 32],
        address: AccountId32,
    ) -> Result<Vec<RpcServicesWithBlueprint>, gadget_common::Error> {
        let call = api::apis()
            .services_api()
            .query_services_with_blueprints_by_operator(address);
        let block_ref = BlockRef::from_hash(H256::from_slice(&at));
        //let call = api::storage().services().next_blueprint_id();
        let ret: Vec<QueryServicesWithBlueprintsByOperator> = self
            .rpc_client
            .storage()
            .at(block_ref.clone())
            .fetch_or_default(&call)
            .await
            .map_err(|e| gadget_common::Error::ClientError { err: e.to_string() })?;

        Ok(ret)
    }
}
