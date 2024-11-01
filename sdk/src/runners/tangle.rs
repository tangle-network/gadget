use crate::config::ProtocolSpecificSettings;
use crate::{config::GadgetConfiguration, runners::RunnerError};
use crate::{info, tx};
use sp_core::Pair;
use subxt::tx::Signer;
use tangle_subxt::tangle_testnet_runtime::api;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::PriceTargets as TanglePriceTargets;

use super::BlueprintConfig;

/// Wrapper for `tangle_subxt`'s [`PriceTargets`]
///
/// This provides a [`Default`] impl for a zeroed-out [`PriceTargets`].
///
/// [`PriceTargets`]: tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::PriceTargets
#[derive(Clone)]
pub struct PriceTargets(TanglePriceTargets);

impl From<TanglePriceTargets> for PriceTargets {
    fn from(t: TanglePriceTargets) -> Self {
        PriceTargets(t)
    }
}

impl Default for PriceTargets {
    fn default() -> Self {
        Self(TanglePriceTargets {
            cpu: 0,
            mem: 0,
            storage_hdd: 0,
            storage_ssd: 0,
            storage_nvme: 0,
        })
    }
}

#[derive(Clone, Default)]
pub struct TangleConfig {
    pub price_targets: PriceTargets,
}

#[async_trait::async_trait]
impl BlueprintConfig for TangleConfig {
    async fn requires_registration(
        &self,
        env: &GadgetConfiguration<parking_lot::RawRwLock>,
    ) -> Result<bool, RunnerError> {
        // Get the blueprint_id from the Tangle protocol specific settings
        let blueprint_id = match &env.protocol_specific {
            ProtocolSpecificSettings::Tangle(settings) => {
                if settings.skip_registration {
                    return Ok(false);
                }
                settings.blueprint_id
            }
            _ => {
                return Err(RunnerError::InvalidProtocol(
                    "Expected Tangle protocol".into(),
                ))
            }
        };

        // Check if the operator is already registered
        let client = env.client().await?;
        let signer = env.first_sr25519_signer()?;
        let account_id = signer.account_id();

        let operator_profile_query = api::storage().services().operators_profile(account_id);
        let operator_profile = client
            .storage()
            .at_latest()
            .await?
            .fetch(&operator_profile_query)
            .await?
            .ok_or_else(|| RunnerError::StorageError("Operator profile not found".into()))?;
        let is_registered = operator_profile
            .blueprints
            .0
            .iter()
            .any(|&id| id == blueprint_id);

        Ok(!is_registered)
    }

    async fn register(
        &self,
        env: &GadgetConfiguration<parking_lot::RawRwLock>,
    ) -> Result<(), RunnerError> {
        let client = env.client().await?;
        let signer = env.first_sr25519_signer()?;
        let ecdsa_pair = env.first_ecdsa_signer()?;

        // Parse Tangle protocol specific settings
        let ProtocolSpecificSettings::Tangle(blueprint_settings) = &env.protocol_specific else {
            return Err(RunnerError::InvalidProtocol(
                "Expected Tangle protocol".into(),
            ));
        };
        let blueprint_id = blueprint_settings.blueprint_id;

        let xt = api::tx().services().register(
            blueprint_id,
            services::OperatorPreferences {
                key: ecdsa_pair.signer().public().0,
                price_targets: self.price_targets.clone().0,
            },
            Default::default(),
        );

        // send the tx to the tangle and exit.
        let result = tx::tangle::send(&client, &signer, &xt).await?;
        info!("Registered operator with hash: {:?}", result);
        Ok(())
    }
}
