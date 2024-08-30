use gadget_common::config::DebugLogger;
use gadget_common::tangle_runtime::api::runtime_types::tangle_primitives::services;
use gadget_common::tangle_runtime::api::runtime_types::tangle_primitives::services::ServiceBlueprint;
use gadget_common::tangle_runtime::api::DispatchError;
use gadget_common::tangle_runtime::AccountId32;
use gadget_common::tangle_subxt::subxt::backend::BlockRef;
use gadget_common::tangle_subxt::subxt::utils::H256;
use gadget_common::tangle_subxt::subxt::{Config, OnlineClient};
use gadget_common::tangle_subxt::tangle_testnet_runtime::api;
use parity_scale_codec::Encode;

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
    ) -> Result<Option<ServiceBlueprint>, gadget_common::Error> {
        let call = api::storage().services().blueprints(blueprint_id);
        let at = BlockRef::from_hash(H256::from_slice(&at));
        let ret: Option<ServiceBlueprint> = self
            .rpc_client
            .storage()
            .at(at)
            .fetch(&call)
            .await
            .map_err(|e| gadget_common::Error::ClientError { err: e.to_string() })?
            .map(|r| r.1);

        Ok(ret)
    }

    pub async fn query_operator_blueprints(
        &self,
        at_block: [u8; 32],
        address: AccountId32,
    ) -> Result<Vec<RpcServicesWithBlueprint>, gadget_common::Error> {
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
            .map_err(|e| gadget_common::Error::ClientError { err: e.to_string() })?
            .map_err(|err| self.dispatch_error_to_gadget_common_error(err, &at_block))?;

        Ok(ret)
    }

    pub fn dispatch_error_to_gadget_common_error(
        &self,
        err: DispatchError,
        at: &[u8; 32],
    ) -> gadget_common::Error {
        let metadata = self.rpc_client.metadata();
        let at_hex = hex::encode(at);
        let dispatch_error = gadget_common::tangle_subxt::subxt::error::DispatchError::decode_from(
            err.encode(),
            metadata,
        );
        match dispatch_error {
            Ok(dispatch_error) => gadget_common::Error::DispatchError {
                err: format!("{dispatch_error}"),
            },
            Err(err) => gadget_common::Error::DispatchError {
                err: format!("Failed to construct DispatchError at block 0x{at_hex}: {err}"),
            },
        }
    }
}
