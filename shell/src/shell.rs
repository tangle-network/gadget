use std::sync::Arc;

use crate::keystore::KeystoreContainer;
use color_eyre::eyre::OptionExt;
use gadget_common::{
    client::{PairSigner, SubxtPalletSubmitter},
    config::{DebugLogger, PrometheusConfig},
    full_protocol::NodeInput,
    keystore::{ECDSAKeyStore, InMemoryBackend},
};
use sp_application_crypto::Ss58Codec;
use sp_core::{crypto::KeyTypeId, ecdsa, sr25519, ByteArray, Pair};
use sp_keystore::Keystore;

use crate::tangle::crypto;
use tangle_subxt::subxt;

const KEY_TYPE: KeyTypeId = KeyTypeId(*b"test");

/// Start the shell and run it forever
#[tracing::instrument(skip(config))]
pub async fn run_forever(config: crate::config::ShellConfig) -> color_eyre::Result<()> {
    let (role_key, acco_key) = load_keys_from_keystore(&config.keystore)?;
    let subxt_client =
        subxt::OnlineClient::<subxt::PolkadotConfig>::from_url(config.subxt.endpoint).await?;
    let tangle = crate::tangle::TangleRuntime::new(subxt_client.clone());
    let logger = DebugLogger {
        peer_id: role_key.public().to_ss58check(),
    };
    let pair_signer = PairSigner::new(acco_key.clone());
    let pallet_tx_submitter = SubxtPalletSubmitter::with_client(subxt_client, pair_signer, logger);
    let wrapped_keystore = ECDSAKeyStore::new(InMemoryBackend::new(), role_key);
    let node_input = NodeInput {
        clients: vec![tangle],
        account_id: acco_key.public(),
        logger,
        pallet_tx: Arc::new(pallet_tx_submitter),
        keystore: wrapped_keystore,
        node_index: 0,
        additional_params: (),
        prometheus_config: PrometheusConfig::Disabled,
        // TODO(tbraun96): Add network configuration
        networks: todo!("Network configuration"),
    };
    Ok(())
}

pub fn load_keys_from_keystore(
    keystore_config: &crate::config::KeystoreConfig,
) -> color_eyre::Result<(ecdsa::Pair, sr25519::Pair)> {
    let keystore_container = KeystoreContainer::new(keystore_config)?;
    let keystore = keystore_container.local_keystore();
    tracing::debug!("Loaded keystore from path");
    let ecdsa_keys = keystore.ecdsa_public_keys(crypto::role::KEY_TYPE);
    let sr25519_keys = keystore.sr25519_public_keys(crypto::acco::KEY_TYPE);
    if ecdsa_keys.is_empty() || ecdsa_keys.len() > 1 {
        color_eyre::eyre::bail!(
            "`role`: Expected exactly one key in keystore, found {}",
            ecdsa_keys.len()
        );
    }

    if sr25519_keys.is_empty() || sr25519_keys.len() > 1 {
        color_eyre::eyre::bail!(
            "`acco`: Expected exactly one key in keystore, found {}",
            sr25519_keys.len()
        );
    }
    let role_public_key = crypto::role::Public::from_slice(&ecdsa_keys[0].0)
        .map_err(|_| color_eyre::eyre::eyre!("Failed to parse public key from keystore"))?;
    let account_public_key = crypto::acco::Public::from_slice(&sr25519_keys[0].0)
        .map_err(|_| color_eyre::eyre::eyre!("Failed to parse public key from keystore"))?;
    let role_key = keystore
        .key_pair::<crypto::role::Pair>(&role_public_key)?
        .ok_or_eyre("Failed to load key `role` from keystore")?
        .into_inner();
    let acco_key = keystore
        .key_pair::<crypto::acco::Pair>(&account_public_key)?
        .ok_or_eyre("Failed to load key `acco` from keystore")?
        .into_inner();
    tracing::debug!(%role_public_key, "Loaded key from keystore");
    tracing::debug!(%account_public_key, "Loaded key from keystore");
    Ok((role_key, acco_key))
}
