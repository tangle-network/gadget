use std::sync::Arc;
use std::time::Duration;

use crate::{
    keystore::KeystoreContainer,
    tangle::{TangleConfig, TangleRuntime},
};
use color_eyre::eyre::OptionExt;
use gadget_common::{
    client::{PairSigner, SubxtPalletSubmitter},
    config::{ClientWithApi, DebugLogger, PrometheusConfig},
    full_protocol::NodeInput,
    keystore::{ECDSAKeyStore, InMemoryBackend},
};
use sp_core::{ecdsa, ed25519, sr25519, ByteArray, Pair};
use sp_keystore::Keystore;

use crate::config::ShellConfig;
use crate::network::gossip::GossipHandle;
use crate::tangle::crypto;
use dfns_cggmp21_protocol::constants::{
    DFNS_CGGMP21_KEYGEN_PROTOCOL_NAME, DFNS_CGGMP21_KEYREFRESH_PROTOCOL_NAME,
    DFNS_CGGMP21_KEYROTATE_PROTOCOL_NAME, DFNS_CGGMP21_SIGNING_PROTOCOL_NAME,
};
use gadget_common::keystore::KeystoreBackend;
use tangle_subxt::subxt;

/// The version of the shell
pub const AGENT_VERSION: &str = "tangle/gadget-shell/1.0.0";
pub const CLIENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Start the shell and run it forever
#[tracing::instrument(skip(config))]
pub async fn run_forever(config: ShellConfig) -> color_eyre::Result<()> {
    let (role_key, acco_key) = load_keys_from_keystore(&config.keystore)?;
    let network_key = ed25519::Pair::from_seed(&config.node_key);
    let subxt_client =
        subxt::OnlineClient::<subxt::PolkadotConfig>::from_url(&config.subxt.endpoint).await?;
    let logger = DebugLogger::default();
    let pair_signer = PairSigner::new(acco_key.clone());
    let pallet_tx_submitter =
        SubxtPalletSubmitter::with_client(subxt_client.clone(), pair_signer, logger.clone());
    let wrapped_keystore = ECDSAKeyStore::new(InMemoryBackend::new(), role_key.clone());

    let libp2p_key = libp2p::identity::Keypair::ed25519_from_bytes(network_key.to_raw_vec())
        .map_err(|e| color_eyre::eyre::eyre!("Failed to create libp2p keypair: {e}"))?;

    let (networks, network_task) = crate::network::setup::setup_libp2p_network(
        libp2p_key,
        &config,
        logger.clone(),
        vec![
            DFNS_CGGMP21_KEYGEN_PROTOCOL_NAME,
            DFNS_CGGMP21_SIGNING_PROTOCOL_NAME,
            DFNS_CGGMP21_KEYREFRESH_PROTOCOL_NAME,
            DFNS_CGGMP21_KEYROTATE_PROTOCOL_NAME,
        ],
        role_key.public(),
    )
    .await
    .map_err(|e| color_eyre::eyre::eyre!("Failed to setup network: {e}"))?;

    logger.debug("Successfully initialized network, now waiting for bootnodes to connect ...");
    wait_for_connection_to_bootnodes(&config, &networks, &logger).await?;

    let pallet_tx = Arc::new(pallet_tx_submitter);

    let dfns_cggmp21_protocol = start_protocol(
        vec![
            TangleRuntime::new(subxt_client.clone()),
            TangleRuntime::new(subxt_client.clone()),
            TangleRuntime::new(subxt_client.clone()),
            TangleRuntime::new(subxt_client.clone()),
        ],
        networks,
        acco_key.public(),
        logger,
        pallet_tx,
        wrapped_keystore,
    );

    let ctrl_c = tokio::signal::ctrl_c();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl-C, shutting down");
        }
        dfns = dfns_cggmp21_protocol => {
            if let Err(e) = dfns {
                tracing::error!(?e, "DFNS-CGGMP21 protocol failed");
            }
        }

        _ = network_task => {
            tracing::error!("Network task unexpectedly shutdown")
        }
    }

    Ok(())
}

pub async fn start_protocol<C, KBE>(
    clients: Vec<C>,
    networks: Vec<GossipHandle>,
    account_id: sr25519::Public,
    logger: DebugLogger,
    pallet_tx: Arc<SubxtPalletSubmitter<TangleConfig, PairSigner<TangleConfig>>>,
    keystore: ECDSAKeyStore<KBE>,
) -> color_eyre::Result<()>
where
    C: ClientWithApi + 'static,
    KBE: KeystoreBackend,
{
    let node_input = NodeInput {
        clients,
        account_id,
        logger,
        pallet_tx,
        keystore,
        node_index: 0,
        additional_params: (),
        prometheus_config: PrometheusConfig::Disabled,
        networks,
    };

    // Wait for the protocol to finish
    dfns_cggmp21_protocol::setup_node(node_input).await;

    Err(color_eyre::eyre::eyre!("Protocol finished unexpectedly"))
}

pub fn load_keys_from_keystore(
    keystore_config: &crate::config::KeystoreConfig,
) -> color_eyre::Result<(ecdsa::Pair, sr25519::Pair)> {
    let keystore_container = KeystoreContainer::new(keystore_config)?;
    let keystore = keystore_container.local_keystore();
    tracing::debug!("Loaded keystore from path");
    let ecdsa_keys = keystore.ecdsa_public_keys(crypto::role::KEY_TYPE);
    let sr25519_keys = keystore.sr25519_public_keys(crypto::acco::KEY_TYPE);

    if ecdsa_keys.len() != 1 {
        color_eyre::eyre::bail!(
            "`role`: Expected exactly one key in ECDSA keystore, found {}",
            ecdsa_keys.len()
        );
    }

    if sr25519_keys.len() != 1 {
        color_eyre::eyre::bail!(
            "`acco`: Expected exactly one key in SR25519 keystore, found {}",
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

pub async fn wait_for_connection_to_bootnodes(
    config: &ShellConfig,
    handles: &[GossipHandle],
    logger: &DebugLogger,
) -> color_eyre::Result<()> {
    let n_required = config.bootnodes.len();
    let n_networks = handles.len();
    logger.debug(format!(
        "Waiting for {n_required} peers to show up across {n_networks} networks"
    ));

    let mut tasks = tokio::task::JoinSet::new();

    // For each network, we start a task that checks if we have enough peers connected
    // and then we wait for all of them to finish.

    let wait_for_peers = |handle: GossipHandle, n_required, logger: DebugLogger| async move {
        'inner: loop {
            let n_connected = handle.connected_peers();
            if n_connected >= n_required {
                break 'inner;
            }
            let topic = handle.topic();
            logger.debug(format!("`{topic}`: We currently have {n_connected}/{n_required} peers connected to network"));
            tokio::time::sleep(Duration::from_millis(1000)).await;
        }
    };

    for handle in handles.iter() {
        tasks.spawn(wait_for_peers(handle.clone(), n_required, logger.clone()));
    }
    // Wait for all tasks to finish
    while (tasks.join_next().await).is_some() {}

    Ok(())
}
