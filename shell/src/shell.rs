use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

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
use sp_application_crypto::Ss58Codec;
use sp_core::{ecdsa, ed25519, sr25519, ByteArray, Pair};
use sp_keystore::Keystore;

use crate::config::ShellConfig;
use crate::protocols::dfns;
use crate::tangle::crypto;
use gadget_common::gadget::network::gossip::{GossipHandler, GossipHandlerController};
use gadget_common::prelude::KeystoreBackend;
use libp2p_gadget::config::{
    FullNetworkConfiguration, NetworkConfiguration, NodeKeyConfig, Params, Role, Secret,
    TransportConfig,
};
use libp2p_gadget::peer_store::PeerStore;
use libp2p_gadget::{config::ed25519::SecretKey, NetworkService};
use libp2p_gadget::{NetworkWorker, NotificationService};
use tangle_subxt::subxt;

/// Start the shell and run it forever
#[tracing::instrument(skip(config))]
pub async fn run_forever(config: ShellConfig) -> color_eyre::Result<()> {
    let (role_key, acco_key) = load_keys_from_keystore(&config.keystore)?;
    let network_key = ed25519::Pair::from_seed(&config.node_key);
    let subxt_client =
        subxt::OnlineClient::<subxt::PolkadotConfig>::from_url(&config.subxt.endpoint).await?;
    let logger = DebugLogger {
        peer_id: role_key.public().to_ss58check(),
    };
    let pair_signer = PairSigner::new(acco_key.clone());
    let pallet_tx_submitter =
        SubxtPalletSubmitter::with_client(subxt_client.clone(), pair_signer, logger.clone());
    let wrapped_keystore = ECDSAKeyStore::new(InMemoryBackend::new(), role_key.clone());

    let genesis_hash = config.genesis_hash;
    let (network_worker, peer_store) =
        setup_network_worker(genesis_hash, network_key, &config, dfns::configurator)?;

    let network = network_worker.service().clone();

    let pallet_tx = Arc::new(pallet_tx_submitter);

    let network_worker_handle = tokio::spawn(network_worker.run());
    let peer_store_handle = tokio::spawn(peer_store.run());
    let dfns_cggmp21_protocol = start_protocol(
        vec![
            TangleRuntime::new(subxt_client.clone()),
            TangleRuntime::new(subxt_client.clone()),
            TangleRuntime::new(subxt_client.clone()),
            TangleRuntime::new(subxt_client.clone()),
        ],
        network.clone(),
        acco_key.public(),
        logger,
        pallet_tx,
        wrapped_keystore,
        dfns::gossip_builder,
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
        network_worker = network_worker_handle => {
            if let Err(e) = network_worker {
                tracing::error!(?e, "Network worker failed");
            }
        }
        peer_store = peer_store_handle => {
            if let Err(e) = peer_store {
                tracing::error!(?e, "Peer store failed");
            }
        }
    }
    Ok(())
}

pub async fn start_protocol<C, KBE>(
    clients: Vec<C>,
    network: Arc<NetworkService>,
    account_id: sr25519::Public,
    logger: DebugLogger,
    pallet_tx: Arc<SubxtPalletSubmitter<TangleConfig, PairSigner<TangleConfig>>>,
    keystore: ECDSAKeyStore<KBE>,
    protocol_specific_gossip_builder: impl FnOnce(
        ECDSAKeyStore<KBE>,
        Arc<NetworkService>,
        DebugLogger,
    ) -> color_eyre::Result<(
        Vec<GossipHandler<KBE>>,
        Vec<GossipHandlerController>,
    )>,
) -> color_eyre::Result<()>
where
    C: ClientWithApi + 'static,
    KBE: KeystoreBackend,
{
    // Networks
    let (gossip_handlers, networks) =
        protocol_specific_gossip_builder(keystore.clone(), network.clone(), logger.clone())?;

    let spawn_handles = gossip_handlers
        .into_iter()
        .map(|r| tokio::spawn(r.run()))
        .collect::<Vec<_>>();

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
    let protocol_task = tokio::spawn(dfns_cggmp21_protocol::setup_node(node_input));
    // Wait for the protocol to finish
    protocol_task.await?;
    // if the protocol finishes, we should cancel the network tasks
    for handle in spawn_handles {
        handle.abort();
    }

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

pub const CLIENT_VERSION: &str = "0.0.1";

fn setup_network_worker(
    genesis_hash: [u8; 32],
    identity: ed25519::Pair,
    config: &ShellConfig,
    protocol_specific_configurator: impl FnOnce(
        &mut FullNetworkConfiguration,
    ) -> Vec<Box<dyn NotificationService>>,
) -> color_eyre::Result<(NetworkWorker, PeerStore)> {
    // Step 1: Derive Secret Key from Identity
    let secret_key_bytes = identity.seed().to_vec();
    let secret_key = SecretKey::try_from_bytes(secret_key_bytes)
        .map_err(|e| color_eyre::eyre::eyre!("Failed to create secret key from identity: {e}"))?;
    let node_key = NodeKeyConfig::Ed25519(Secret::Input(secret_key));
    let node_name = identity.public().to_ss58check();

    // Step 2: Create Network Configuration
    let mut network_config = NetworkConfiguration::new(node_name, CLIENT_VERSION, node_key, None);
    let listen_addr = format!("/ip4/{}/tcp/{}", config.bind_ip, config.bind_port).parse()?;
    network_config.listen_addresses = vec![listen_addr];
    network_config.boot_nodes = config.bootnodes.clone();
    if cfg!(debug_assertions) {
        network_config.transport = TransportConfig::Normal {
            enable_mdns: true,
            allow_private_ip: true,
        };
    }

    // Step 3: Create Full Network Configuration
    let mut full_network_config = FullNetworkConfiguration::new(&network_config);
    let _notification_services = protocol_specific_configurator(&mut full_network_config);
    let bootnodes = config.bootnodes.iter().map(|addr| addr.peer_id).collect();
    let peer_store = PeerStore::new(bootnodes);

    let params = Params {
        role: Role::Authority,
        executor: Box::new(tokio_executor),
        network_config: full_network_config,
        peer_store: peer_store.handle(),
        protocol_id: "/tangle/gadget-shell/1".into(),
        genesis_hash,
        fork_id: None,
        metrics_registry: None,
    };

    let network_worker = NetworkWorker::new(params)
        .map_err(|e| color_eyre::eyre::eyre!("Failed to create network worker: {e}"))?;

    Ok((network_worker, peer_store))
}

fn tokio_executor(future: Pin<Box<dyn Future<Output = ()> + Send>>) {
    tokio::spawn(future);
}
