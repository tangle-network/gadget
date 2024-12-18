use crate::error::Result;
use gadget_config::GadgetConfiguration;
use subxt::utils::AccountId32;
use tangle_subxt::tangle_testnet_runtime::api;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::pallet_multi_asset_delegation::types::operator::OperatorMetadata;

pub struct TangleClient {
    pub config: GadgetConfiguration,
    call_id: Option<u64>,
    rpc_client: Option<subxt::OnlineClient<subxt::PolkadotConfig>>,
}

impl TangleClient {
    /// Create a new [`TangleClient`] from a [`GadgetConfiguration`]
    pub async fn new(config: GadgetConfiguration) -> Self {
        Self {
            config,
            call_id: None,
            rpc_client: None,
        }
    }

    /// Get the Tangle client from the context.
    pub async fn tangle_client(&mut self) -> Result<&subxt::OnlineClient<subxt::PolkadotConfig>> {
        if self.rpc_client.is_none() {
            let rpc_url = self.config.ws_rpc_endpoint.as_str();
            let client = subxt::OnlineClient::from_url(rpc_url).await?;
            self.rpc_client = Some(client);
        }
        Ok(self.rpc_client.as_ref().unwrap())
    }

    /// Get [`metadata`](OperatorMetadata) for an operator by [`Account ID`](AccountId32)
    async fn operator_metadata(
        &self,
        client: &subxt::OnlineClient<subxt::PolkadotConfig>,
        operator: AccountId32,
    ) -> Result<
        Option<
            OperatorMetadata<
                AccountId32,
                api::assets::events::burned::Balance,
                api::assets::events::accounts_destroyed::AssetId,
                api::runtime_types::tangle_testnet_runtime::MaxDelegations,
                api::runtime_types::tangle_testnet_runtime::MaxOperatorBlueprints,
            >,
        >,
    > {
        let storage = client.storage().at_latest().await?;
        let metadata_storage_key = api::storage().multi_asset_delegation().operators(operator);
        storage
            .fetch(&metadata_storage_key)
            .await
            .map_err(Into::into)
    }

    fn get_call_id(&mut self) -> &mut Option<u64> {
        &mut self.call_id
    }

    fn set_call_id(&mut self, call_id: u64) {
        *self.get_call_id() = Some(call_id);
    }
}
