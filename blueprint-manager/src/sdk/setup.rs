use std::collections::HashMap;
use std::time::Duration;

use gadget_io::tokio::task::JoinHandle;
use gadget_sdk::clients::tangle::runtime::TangleRuntimeClient;
use gadget_sdk::network::Network;
use gadget_sdk::prometheus::PrometheusConfig;
use gadget_sdk::store::{ECDSAKeyStore, KeyValueStoreBackend};
use sp_core::{keccak_256, sr25519, Pair};

use crate::sdk::config::SingleGadgetConfig;
pub use gadget_io::KeystoreContainer;
use gadget_io::SubstrateKeystore;
use gadget_sdk::debug;
use gadget_sdk::network::gossip::GossipHandle;
use gadget_sdk::network::setup::NetworkConfig;
use itertools::Itertools;
use libp2p::Multiaddr;

/// Used for constructing an instance of a node.
///
/// If there is both a keygen and a signing protocol, then the length of the vectors are 2.
/// The length of the vector is equal to the numbers of protocols that the constructed node
/// is going to concurrently execute.
pub struct NodeInput<N: Network, KBE: KeyValueStoreBackend, D> {
    pub clients: Vec<TangleRuntimeClient>,
    pub networks: Vec<N>,
    pub account_id: sr25519::Public,
    pub keystore: ECDSAKeyStore<KBE>,
    pub node_index: usize,
    pub additional_params: D,
    pub prometheus_config: PrometheusConfig,
}

pub type SingleGadgetInput<KBE> = NodeInput<GossipHandle, KBE, ()>;

/// Generates the NodeInput and handle to the networking layer for the given config.
#[tracing::instrument(skip(config))]
pub async fn generate_node_input<KBE: KeyValueStoreBackend>(
    config: SingleGadgetConfig<KBE>,
) -> color_eyre::Result<(SingleGadgetInput<KBE>, JoinHandle<()>)> {
    let keystore_config = KeystoreContainer::new(&config.keystore)?;
    let (ecdsa_key, acco_key) = (keystore_config.ecdsa_key()?, keystore_config.sr25519_key()?);
    //let network_key = ed25519::Pair::from_seed(&config.node_key).to_raw_vec();
    let keystore = ECDSAKeyStore::new(config.keystore_backend.clone(), ecdsa_key.clone());

    // Use the first 32 bytes of the sr25519 account key as the network key. We discard the 32 remaining nonce seed bytes
    // thus ensuring that the network key, when used, will actually have slightly different properties than the original
    // key since the nonces will not be the same.
    let network_key = &mut acco_key.as_ref().to_half_ed25519_bytes()[..32];
    let libp2p_key = libp2p::identity::Keypair::ed25519_from_bytes(network_key)
        .map_err(|e| color_eyre::eyre::eyre!("Failed to create libp2p keypair: {e}"))?;

    // Create a network for each subprotocol in the protocol (e.g., keygen, signing, refresh, rotate, = 4 total subprotocols = n_protocols)
    let network_ids = (0..config.n_protocols)
        .map(|_| format!("{:?}", config.services[0]))
        .map(|r| keccak_256(r.as_bytes()))
        .map(hex::encode)
        .enumerate()
        .map(|(id, r)| format!("/tangle/{r}-{id}/1.0.0"))
        .sorted()
        .collect::<Vec<_>>();

    let libp2p_config = NetworkConfig::new(
        libp2p_key,
        ecdsa_key,
        config.bootnodes.clone(),
        config.bind_ip,
        config.bind_port,
        network_ids.clone(),
    );

    let (networks, network_task) =
        gadget_sdk::network::setup::multiplexed_libp2p_network(libp2p_config)
            .map_err(|e| color_eyre::eyre::eyre!("Failed to setup network: {e}"))?;

    debug!("Successfully initialized network, now waiting for bootnodes to connect ...");
    wait_for_connection_to_bootnodes(&config.bootnodes, &networks).await?;

    let client =
        TangleRuntimeClient::from_url(&config.bind_ip.to_string(), acco_key.public().0.into())
            .await?;
    let networks = networks
        .into_iter()
        .sorted_by_key(|r| r.0.clone())
        .map(|r| r.1)
        .collect::<Vec<_>>();
    let clients = (0..networks.len())
        .map(|_| client.clone())
        .collect::<Vec<_>>();

    let node_input = NodeInput::<GossipHandle, KBE, ()> {
        clients,
        account_id: acco_key.public(),
        keystore,
        node_index: 0,
        additional_params: (),
        prometheus_config: PrometheusConfig::Disabled,
        networks,
    };

    Ok((node_input, network_task))
}

pub async fn wait_for_connection_to_bootnodes(
    bootnodes: &[Multiaddr],
    handles: &HashMap<String, GossipHandle>,
) -> color_eyre::Result<()> {
    let n_required = bootnodes.len();
    let n_networks = handles.len();

    debug!("Waiting for {n_required} peers to show up across {n_networks} networks");

    let mut tasks = gadget_io::tokio::task::JoinSet::new();

    // For each network, we start a task that checks if we have enough peers connected
    // and then we wait for all of them to finish.

    let wait_for_peers = |handle: GossipHandle, n_required| async move {
        'inner: loop {
            let n_connected = handle.connected_peers();
            if n_connected >= n_required {
                break 'inner;
            }
            let topic = handle.topic();
            debug!("`{topic}`: We currently have {n_connected}/{n_required} peers connected to network");
            gadget_io::tokio::time::sleep(Duration::from_millis(1000)).await;
        }
    };

    for handle in handles.values() {
        tasks.spawn(wait_for_peers(handle.clone(), n_required));
    }
    // Wait for all tasks to finish
    while tasks.join_next().await.is_some() {}

    Ok(())
}
