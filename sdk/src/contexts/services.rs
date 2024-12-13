use async_trait::async_trait;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::{Service, ServiceBlueprint};
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::sp_arithmetic::per_things::Percent;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::pallet_multi_asset_delegation::types::operator::OperatorMetadata;
use tangle_subxt::tangle_testnet_runtime::api::assets::events::burned::Balance;
use tangle_subxt::tangle_testnet_runtime::api::assets::events::accounts_destroyed::AssetId;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_testnet_runtime::{MaxDelegations, MaxDelegatorBlueprints, MaxOperatorBlueprints, MaxUnstakeRequests, MaxWithdrawRequests};
use tangle_subxt::tangle_testnet_runtime::api::system::storage::types::number::Number;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::pallet_multi_asset_delegation::types::delegator::DelegatorMetadata;

/// `ServicesContext` trait provides access to the current service and current blueprint from the context.
#[allow(clippy::type_complexity)]
#[async_trait]
pub trait ServicesContext {
    type Config: subxt::Config;
    /// Get the current blueprint information from the context.
    async fn current_blueprint(
        &self,
        client: &subxt::OnlineClient<Self::Config>,
    ) -> color_eyre::Result<ServiceBlueprint, subxt::Error>;

    /// Query the current blueprint owner from the context.
    async fn current_blueprint_owner(
        &self,
        client: &subxt::OnlineClient<Self::Config>,
    ) -> color_eyre::Result<subxt::utils::AccountId32, subxt::Error>;

    /// Get the current service operators with their restake exposure  from the context.
    /// This function will return a list of service operators that are selected to run this service
    /// instance.
    async fn current_service_operators(
        &self,
        client: &subxt::OnlineClient<Self::Config>,
    ) -> color_eyre::Result<Vec<(subxt::utils::AccountId32, Percent)>, subxt::Error>;

    #[allow(clippy::type_complexity)]
    /// Get metadata for a list of operators from the context.
    async fn operators_metadata(
        &self,
        client: &subxt::OnlineClient<Self::Config>,
        operators: Vec<subxt::utils::AccountId32>,
    ) -> color_eyre::Result<
        Vec<(
            subxt::utils::AccountId32,
            OperatorMetadata<
                subxt::utils::AccountId32,
                Balance,
                AssetId,
                MaxDelegations,
                MaxOperatorBlueprints,
            >,
        )>,
        subxt::Error,
    >;

    /// Get metadata for a single operator from the context.
    /// This function will return the metadata for a single operator.
    async fn operator_metadata(
        &self,
        client: &subxt::OnlineClient<Self::Config>,
        operator: subxt::utils::AccountId32,
    ) -> color_eyre::Result<
        Option<
            OperatorMetadata<
                subxt::utils::AccountId32,
                Balance,
                AssetId,
                MaxDelegations,
                MaxOperatorBlueprints,
            >,
        >,
        subxt::Error,
    >;

    /// Get the current service instance from the context.
    async fn service_instance(
        &self,
        client: &subxt::OnlineClient<Self::Config>,
    ) -> color_eyre::Result<
        Service<subxt::utils::AccountId32, Number, AssetId>,
        subxt::Error,
    >;

    #[allow(clippy::type_complexity)]
    /// Get delegations for a list of operators from the context.
    async fn operator_delegations(
        &self,
        client: &subxt::OnlineClient<Self::Config>,
        operators: Vec<subxt::utils::AccountId32>,
    ) -> color_eyre::Result<
        Vec<(
            subxt::utils::AccountId32, // operator
            Option<
                DelegatorMetadata<
                    subxt::utils::AccountId32,
                    AssetId,
                    Balance,
                    MaxWithdrawRequests,
                    MaxDelegations,
                    MaxUnstakeRequests,
                    MaxDelegatorBlueprints,
                >,
            >,
        )>,
        subxt::Error,
    >;

    /// Get delegations for a single operator from the context.
    async fn operator_delegation(
        &self,
        client: &subxt::OnlineClient<Self::Config>,
        operator: subxt::utils::AccountId32,
    ) -> color_eyre::Result<
        Option<
            DelegatorMetadata<
                subxt::utils::AccountId32,
                AssetId,
                Balance,
                MaxWithdrawRequests,
                MaxDelegations,
                MaxUnstakeRequests,
                MaxDelegatorBlueprints,
            >,
        >,
        subxt::Error,
    >;
}
