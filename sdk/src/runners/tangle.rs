use crate::{config::GadgetConfiguration, runners::RunnerError};
use crate::{info, tx};
use sp_core::Pair;
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
    async fn register(
        &self,
        env: &GadgetConfiguration<parking_lot::RawRwLock>,
    ) -> Result<(), RunnerError> {
        let client = env.client().await?;
        let signer = env.first_sr25519_signer()?;
        let ecdsa_pair = env.first_ecdsa_signer()?;

        let xt = api::tx().services().register(
            env.blueprint_id,
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
