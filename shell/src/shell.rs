use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::keystore::KeystoreContainer;
use color_eyre::eyre::OptionExt;
use gadget_common::{
    client::{PairSigner, SubxtPalletSubmitter},
    config::{DebugLogger, PrometheusConfig},
    full_protocol::NodeInput,
    keystore::{ECDSAKeyStore, InMemoryBackend},
};
use sc_utils::mpsc::tracing_unbounded;
use sp_application_crypto::{AppCrypto, Ss58Codec};
use sp_core::{crypto::KeyTypeId, ecdsa, ed25519, sr25519, ByteArray, Pair};
use sp_keystore::Keystore;

use crate::config::ShellConfig;
use crate::tangle::crypto;
use gadget_common::gadget::network::gossip::{GossipHandlerController, NetworkGossipEngineBuilder};
use gadget_common::prelude::{Block, KeystoreBackend, SyncingService};
use libp2p_gadget::config::ed25519::SecretKey;
use libp2p_gadget::config::{
    FullNetworkConfiguration, NetworkConfiguration, NodeKeyConfig, NonDefaultSetConfig, Params,
    Role, Secret, SetConfig,
};
use libp2p_gadget::peer_store::PeerStore;
use libp2p_gadget::{NetworkWorker, ProtocolName};
use tangle_subxt::subxt;

const KEY_TYPE: KeyTypeId = KeyTypeId(*b"test");

/// Start the shell and run it forever
#[tracing::instrument(skip(config))]
pub async fn run_forever(config: ShellConfig) -> color_eyre::Result<()> {
    let (role_key, acco_key, network_key) = load_keys_from_keystore(&config.keystore)?;
    let subxt_client =
        subxt::OnlineClient::<subxt::PolkadotConfig>::from_url(config.subxt.endpoint).await?;
    let tangle = crate::tangle::TangleRuntime::new(subxt_client.clone());
    let logger = DebugLogger {
        peer_id: role_key.public().to_ss58check(),
    };
    let pair_signer = PairSigner::new(acco_key.clone());
    let pallet_tx_submitter =
        SubxtPalletSubmitter::with_client(subxt_client, pair_signer, logger.clone());
    let wrapped_keystore = ECDSAKeyStore::new(InMemoryBackend::new(), role_key.clone());

    let genesis_hash = config.genesis_hash;
    let gossip_network = setup_network::<InMemoryBackend, _>(
        wrapped_keystore.clone(),
        logger.clone(),
        genesis_hash,
        network_key,
        &config,
    )?;

    let node_input = NodeInput {
        clients: vec![tangle],
        account_id: acco_key.public(),
        logger,
        pallet_tx: Arc::new(pallet_tx_submitter),
        keystore: wrapped_keystore,
        node_index: 0,
        additional_params: (),
        prometheus_config: PrometheusConfig::Disabled,
        networks: vec![gossip_network],
    };
    Ok(())
}

pub fn load_keys_from_keystore(
    keystore_config: &crate::config::KeystoreConfig,
) -> color_eyre::Result<(ecdsa::Pair, sr25519::Pair, ed25519::Pair)> {
    let keystore_container = KeystoreContainer::new(keystore_config)?;
    let keystore = keystore_container.local_keystore();
    tracing::debug!("Loaded keystore from path");
    let ecdsa_keys = keystore.ecdsa_public_keys(crypto::role::KEY_TYPE);
    let sr25519_keys = keystore.sr25519_public_keys(crypto::acco::KEY_TYPE);
    let ed25519_keys = keystore.ed25519_public_keys(crypto::acco::KEY_TYPE);

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

    if ed25519_keys.len() != 1 {
        color_eyre::eyre::bail!(
            "`acco`: Expected exactly one key in ED25519 keystore, found {}",
            ed25519_keys.len()
        );
    }

    let role_public_key = crypto::role::Public::from_slice(&ecdsa_keys[0].0)
        .map_err(|_| color_eyre::eyre::eyre!("Failed to parse public key from keystore"))?;
    let account_public_key = crypto::acco::Public::from_slice(&sr25519_keys[0].0)
        .map_err(|_| color_eyre::eyre::eyre!("Failed to parse public key from keystore"))?;
    let ed25519_public_key = crypto::acco::Public::from_slice(&ed25519_keys[0].0)
        .map_err(|_| color_eyre::eyre::eyre!("Failed to parse public key from keystore"))?;
    let role_key = keystore
        .key_pair::<crypto::role::Pair>(&role_public_key)?
        .ok_or_eyre("Failed to load key `role` from keystore")?
        .into_inner();
    let acco_key = keystore
        .key_pair::<crypto::acco::Pair>(&account_public_key)?
        .ok_or_eyre("Failed to load key `acco` from keystore")?
        .into_inner();

    let ed25519_public_key =
        <sp_application_crypto::ed25519::AppPair as AppCrypto>::Public::from_slice(
            ed25519_public_key.as_slice(),
        )
        .map_err(|_| color_eyre::eyre::eyre!("Failed to parse public key from keystore"))?;

    let ed25519_key = keystore
        .key_pair::<sp_application_crypto::ed25519::AppPair>(&ed25519_public_key)?
        .ok_or_eyre("Failed to load key `acco` from keystore")?
        .into_inner();
    tracing::debug!(%role_public_key, "Loaded key from keystore");
    tracing::debug!(%account_public_key, "Loaded key from keystore");
    Ok((role_key, acco_key, ed25519_key))
}

pub const PROTOCOL_NAME: &'static str = "webb-gadget-gossip-protocol";
pub const FORK_ID: &'static str = "webb-gadget-gossip-protocol-0.0.1";
pub const CLIENT_VERSION: &'static str = "0.0.1";

fn setup_network<KBE: KeystoreBackend, B: Block + 'static>(
    key_store: ECDSAKeyStore<KBE>,
    logger: DebugLogger,
    genesis_hash: B::Hash,
    identity: ed25519::Pair,
    config: &ShellConfig,
) -> color_eyre::Result<GossipHandlerController> {
    // Step 1: Derive Secret Key from Identity
    let secret_key_bytes = identity.seed().to_vec();
    let secret_key = SecretKey::try_from_bytes(secret_key_bytes)
        .map_err(|e| color_eyre::eyre::eyre!("Failed to create secret key from identity: {e}"))?;
    let node_key = NodeKeyConfig::Ed25519(Secret::Input(secret_key));
    let node_name = identity.public().to_ss58check();
    let protocol_name: ProtocolName = PROTOCOL_NAME.to_string().into();
    let fork_id = FORK_ID.to_string();

    // Step 2: Create Network Configuration
    let mut network_config = NetworkConfiguration::new(node_name, CLIENT_VERSION, node_key, None);
    /*let listen_addr = format!("/ip4/{}/tcp/{}", config.bind_ip, config.bind_port)
    .parse()
    .unwrap();*/
    let listen_addr = format!("/ip4/{}/udp/{}/quic-v1", config.bind_ip, config.bind_port)
        .parse()
        .unwrap();
    network_config.listen_addresses = vec![listen_addr];

    // Step 3: Create Full Network Configuration
    let full_network_config = FullNetworkConfiguration::new(&network_config);
    let set_config = SetConfig::default();

    // Step 4: Create NonDefaultSetConfig and Create Peer Store
    let (block_announce_config, _) = NonDefaultSetConfig::new(
        protocol_name.clone(),
        vec![],
        1024 * 1024 * 8,
        None,
        set_config,
    );

    let bootnodes = config.bootnodes.iter().map(|addr| addr.peer_id).collect();
    let peer_store = PeerStore::new(bootnodes);

    let params = Params::<B> {
        role: Role::Authority,
        executor: Box::new(tokio_executor),
        network_config: full_network_config,
        peer_store: peer_store.handle(),
        protocol_id: PROTOCOL_NAME.into(),
        genesis_hash,
        fork_id: Some(fork_id),
        metrics_registry: None,
        block_announce_config,
    };

    let network_worker = NetworkWorker::<B, B::Hash>::new(params)
        .map_err(|e| color_eyre::eyre::eyre!("Failed to create network worker: {e}"))?;
    let network_service = network_worker.service().clone();
    let (tx, rx) = tracing_unbounded("gossip", 1024);
    drop(rx);

    let syncing_service = SyncingService::<B>::new(
        tx,
        Arc::new(Default::default()),
        Arc::new(Default::default()),
    );
    let (_, controller) = NetworkGossipEngineBuilder::new(PROTOCOL_NAME.into(), key_store)
        .build::<B>(network_service, Arc::new(syncing_service), None, logger)?;
    controller.set_gossip_enabled(true);

    Ok(controller)
}

fn tokio_executor(future: Pin<Box<dyn Future<Output = ()> + Send>>) {
    tokio::spawn(future);
}
