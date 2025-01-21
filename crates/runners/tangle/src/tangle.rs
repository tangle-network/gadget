use crate::error::TangleError;
use futures::future::select_ok;
use gadget_clients::tangle;
use gadget_config::{GadgetConfiguration, ProtocolSettings};
use gadget_crypto::sp_core::{SpEcdsa, SpSr25519};
use gadget_keystore::backends::Backend;
use gadget_keystore::{Keystore, KeystoreConfig};
use gadget_runner_core::config::BlueprintConfig;
use gadget_runner_core::error::{RunnerError as Error, RunnerError};
use gadget_std::string::ToString;
use k256::elliptic_curve::sec1::ToEncodedPoint;
use subxt_core::config::PolkadotConfig;
use subxt_core::tx::signer::PairSigner;
use tangle_subxt::subxt_core;
use tangle_subxt::tangle_testnet_runtime::api;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::PriceTargets as TanglePriceTargets;
use tangle_subxt::tangle_testnet_runtime::api::services::calls::types::register::RegistrationArgs;

/// Wrapper for `tangle_subxt`'s [`PriceTargets`]
///
/// This provides a [`Default`] impl for a zeroed-out [`PriceTargets`].
///
/// [`PriceTargets`]: tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::PriceTargets
#[derive(Clone)]
pub struct PriceTargets(pub TanglePriceTargets);

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
    async fn requires_registration(&self, env: &GadgetConfiguration) -> Result<bool, Error> {
        requires_registration_impl(env).await
    }

    async fn register(&self, env: &GadgetConfiguration) -> Result<(), Error> {
        register_impl(self.clone().price_targets, vec![], env).await
    }
}

pub async fn requires_registration_impl(env: &GadgetConfiguration) -> Result<bool, Error> {
    let blueprint_id = match env.protocol_settings {
        ProtocolSettings::Tangle(settings) => settings.blueprint_id,
        _ => {
            return Err(RunnerError::InvalidProtocol(
                "Expected Tangle protocol".into(),
            ))
        }
    };

    // Check if the operator is already registered
    let client = get_client(env.ws_rpc_endpoint.as_str(), env.http_rpc_endpoint.as_str()).await?;

    // TODO: Improve key fetching logic
    let keystore_path = env.clone().keystore_uri;
    let keystore_config = KeystoreConfig::new()
        .in_memory(false)
        .fs_root(keystore_path);
    let keystore = Keystore::new(keystore_config).unwrap();

    // TODO: Key IDs
    let sr25519_key = keystore
        .first_local::<SpSr25519>()
        .map_err(TangleError::from)?;
    let sr25519_pair = keystore.get_secret::<SpSr25519>(&sr25519_key).unwrap();
    let signer: PairSigner<PolkadotConfig, sp_core::sr25519::Pair> =
        PairSigner::new(sr25519_pair.0);

    let account_id = signer.account_id();

    let operator_profile_query = api::storage().services().operators_profile(account_id);
    let operator_profile = client
        .storage()
        .at_latest()
        .await
        .map_err(|e| <TangleError as Into<RunnerError>>::into(TangleError::Network(e.to_string())))?
        .fetch(&operator_profile_query)
        .await
        .map_err(|e| {
            <TangleError as Into<RunnerError>>::into(TangleError::Network(e.to_string()))
        })?;
    let is_registered = operator_profile
        .map(|p| p.blueprints.0.iter().any(|&id| id == blueprint_id))
        .unwrap_or(false);

    Ok(!is_registered)
}

pub async fn register_impl(
    price_targets: PriceTargets,
    registration_args: RegistrationArgs,
    env: &GadgetConfiguration,
) -> Result<(), RunnerError> {
    let client = get_client(env.ws_rpc_endpoint.as_str(), env.http_rpc_endpoint.as_str()).await?;

    // TODO: Improve key fetching logic
    let keystore_path = env.clone().keystore_uri;
    let keystore_config = KeystoreConfig::new()
        .in_memory(false)
        .fs_root(keystore_path);
    let keystore = Keystore::new(keystore_config).unwrap();

    // TODO: Key IDs
    let sr25519_key = keystore
        .first_local::<SpSr25519>()
        .map_err(TangleError::from)?;
    let sr25519_pair = keystore.get_secret::<SpSr25519>(&sr25519_key).unwrap();
    let signer = PairSigner::new(sr25519_pair.0);

    let ecdsa_key = keystore
        .first_local::<SpEcdsa>()
        .map_err(TangleError::from)?;

    // Parse Tangle protocol specific settings
    let ProtocolSettings::Tangle(blueprint_settings) = env.protocol_settings else {
        return Err(RunnerError::InvalidProtocol(
            "Expected Tangle protocol".into(),
        ));
    };

    let account_id = signer.account_id();
    // Check if the operator is active operator.
    let operator_active_query = api::storage()
        .multi_asset_delegation()
        .operators(account_id);
    let operator_active = client
        .storage()
        .at_latest()
        .await
        .map_err(|e| <TangleError as Into<RunnerError>>::into(TangleError::Network(e.to_string())))?
        .fetch(&operator_active_query)
        .await
        .map_err(|e| {
            <TangleError as Into<RunnerError>>::into(TangleError::Network(e.to_string()))
        })?;
    if operator_active.is_none() {
        return Err(RunnerError::NotActiveOperator);
    }

    let blueprint_id = blueprint_settings.blueprint_id;

    let uncompressed_pk = decompress_pubkey(&ecdsa_key.0 .0).ok_or_else(|| {
        RunnerError::Other("Unable to convert compressed ECDSA key to uncompressed key".to_string())
    })?;

    let xt = api::tx().services().register(
        blueprint_id,
        services::OperatorPreferences {
            key: uncompressed_pk,
            price_targets: price_targets.clone().0,
        },
        registration_args,
        0,
    );

    // send the tx to the tangle and exit.
    let result = gadget_utils::tangle::tx::send(&client, &signer, &xt)
        .await
        .map_err(|e| {
            <TangleError as Into<RunnerError>>::into(TangleError::Network(e.to_string()))
        })?;
    gadget_logging::info!("Registered operator with hash: {:?}", result);
    Ok(())
}

// TODO: Push this upstream: https://docs.rs/sp-core/latest/src/sp_core/ecdsa.rs.html#59-74
pub fn decompress_pubkey(compressed: &[u8; 33]) -> Option<[u8; 65]> {
    // Uncompress the public key
    let pk = k256::PublicKey::from_sec1_bytes(compressed).ok()?;
    let uncompressed = pk.to_encoded_point(false);
    let uncompressed_bytes = uncompressed.as_bytes();

    // Ensure the key has the correct length
    if uncompressed_bytes.len() != 65 {
        return None;
    }

    let mut result = [0u8; 65];
    result.copy_from_slice(uncompressed_bytes);
    Some(result)
}

pub async fn get_client(
    ws_url: &str,
    http_url: &str,
) -> Result<tangle::client::OnlineClient, RunnerError> {
    let task0 = tangle::client::OnlineClient::from_url(ws_url);
    let task1 = tangle::client::OnlineClient::from_url(http_url);
    let client = select_ok([Box::pin(task0), Box::pin(task1)])
        .await
        .map_err(|e| {
            <crate::error::TangleError as Into<RunnerError>>::into(
                crate::error::TangleError::Network(e.to_string()),
            )
        })?
        .0;
    Ok(client)
}
