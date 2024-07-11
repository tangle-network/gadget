use gadget_common::config::DebugLogger;
use gadget_common::tangle_runtime::AccountId32;
use gadget_common::tangle_runtime::api::runtime_types::tangle_primitives::services::ServiceBlueprint;
use gadget_common::tangle_subxt::subxt::{Config, OnlineClient};
use gadget_common::tangle_subxt::subxt::backend::BlockRef;
use gadget_common::tangle_subxt::subxt::utils::H256;
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

impl<C: Config> ServicesClient<C>
where
    BlockRef<<C as Config>::Hash>: From<BlockRef<H256>>,
{
    pub async fn query_restaker_blueprints(
        &self,
        at: [u8; 32],
        address: AccountId32,
    ) -> Result<Vec<ServiceBlueprint>, gadget_common::Error> {
        let block_ref = BlockRef::from_hash(H256::from_slice(&at));
        let call = api::storage().services().next_blueprint_id();
        let next_blueprint_id: u64 = self
            .rpc_client
            .storage()
            .at(block_ref.clone())
            .fetch_or_default(&call)
            .await
            .map_err(|e| gadget_common::Error::ClientError { err: e.to_string() })?;

        let mut ret = vec![];

        for blueprint_id in 0..(next_blueprint_id - 1) {
            let svcs = api::storage().services().blueprints(blueprint_id);
            let blueprint = self
                .rpc_client
                .storage()
                .at(block_ref.clone())
                .fetch(&svcs)
                .await
                .map_err(|e| gadget_common::Error::ClientError { err: e.to_string() })?;

            if let Some(blueprint) = blueprint {
                ret.push(blueprint.1);
            } else {
                self.logger.error(format!(
                    "Failed to fetch blueprint for ID: {}",
                    blueprint_id
                ));
            }
        }

        Ok(ret)
    }
}
