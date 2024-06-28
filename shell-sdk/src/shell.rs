use std::collections::HashMap;
use std::time::Duration;

use crate::keystore::load_keys_from_keystore;

use gadget_common::environments::GadgetEnvironment;
use gadget_common::keystore::KeystoreBackend;
use gadget_common::{
    config::{DebugLogger, PrometheusConfig},
    full_protocol::NodeInput,
    keystore::ECDSAKeyStore,
};
use gadget_io::tokio::task::JoinHandle;
use sp_core::{ed25519, keccak_256, sr25519, Pair};

use crate::config::ShellConfig;
use crate::network::gossip::GossipHandle;
pub use gadget_io::KeystoreContainer;
use itertools::Itertools;

/// The version of the shell-sdk
pub const AGENT_VERSION: &str = "tangle/gadget-shell-sdk/1.0.0";
pub const CLIENT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub type ShellNodeInput<KBE, Env> = NodeInput<Env, GossipHandle, KBE, ()>;

/// Generates the NodeInput and handle to the networking layer for the given config.
#[tracing::instrument(skip(config))]
pub async fn generate_node_input<KBE: KeystoreBackend, Env: GadgetEnvironment>(
    config: ShellConfig<KBE, Env>,
) -> color_eyre::Result<(ShellNodeInput<KBE, Env>, JoinHandle<()>)> {
    let (role_key, acco_key) = load_keys_from_keystore(&config.keystore)?;
    let network_key = ed25519::Pair::from_seed(&config.node_key);
    let logger = DebugLogger::default();
    let wrapped_keystore = ECDSAKeyStore::new(config.keystore_backend.clone(), role_key.clone());

    let libp2p_key = libp2p::identity::Keypair::ed25519_from_bytes(network_key.to_raw_vec())
        .map_err(|e| color_eyre::eyre::eyre!("Failed to create libp2p keypair: {e}"))?;

    // Create a network for each subprotocol in the protocol (e.g., keygen, signing, refresh, rotate, = 4 total subprotocols = n_protocols)
    let network_ids = (0..config.n_protocols)
        .map(|_| format!("{:?}", config.role_types[0]))
        .map(|r| keccak_256(r.as_bytes()))
        .map(hex::encode)
        .enumerate()
        .map(|(id, r)| format!("/tangle/{r}-{id}/1.0.0"))
        .sorted()
        .collect::<Vec<_>>();

    let (networks, network_task) = crate::network::setup::setup_libp2p_network(
        libp2p_key,
        &config,
        logger.clone(),
        network_ids,
        role_key,
    )
    .await
    .map_err(|e| color_eyre::eyre::eyre!("Failed to setup network: {e}"))?;

    logger.debug("Successfully initialized network, now waiting for bootnodes to connect ...");
    wait_for_connection_to_bootnodes(&config, &networks, &logger).await?;

    let node_input = generate_node_input_for_required_protocols(
        &config.environment,
        networks,
        acco_key,
        logger,
        wrapped_keystore,
    )
    .await?;

    Ok((node_input, network_task))
}

pub fn generate_node_input_for_role_group<KBE, Env: GadgetEnvironment>(
    runtime: Env::Client,
    networks: HashMap<String, GossipHandle>,
    account_id: sr25519::Public,
    logger: DebugLogger,
    tx_manager: Env::TransactionManager,
    keystore: ECDSAKeyStore<KBE>,
) -> color_eyre::Result<ShellNodeInput<KBE, Env>>
where
    KBE: KeystoreBackend,
{
    let networks = networks
        .into_iter()
        .sorted_by_key(|r| r.0.clone())
        .map(|r| r.1)
        .collect::<Vec<_>>();
    let clients = (0..networks.len())
        .map(|_| runtime.clone())
        .collect::<Vec<_>>();

    Ok(NodeInput::<Env, GossipHandle, KBE, ()> {
        clients,
        account_id,
        logger,
        tx_manager,
        keystore,
        node_index: 0,
        additional_params: (),
        prometheus_config: PrometheusConfig::Disabled,
        networks,
    })
}

pub async fn generate_node_input_for_required_protocols<KBE, Env: GadgetEnvironment>(
    env: &Env,
    networks: HashMap<String, GossipHandle>,
    acco_key: sr25519::Pair,
    logger: DebugLogger,
    keystore: ECDSAKeyStore<KBE>,
) -> color_eyre::Result<ShellNodeInput<KBE, Env>>
where
    KBE: KeystoreBackend,
{
    let runtime = env
        .setup_runtime()
        .await
        .map_err(|err| color_eyre::Report::msg(format!("Failed to setup runtime: {err}")))?;
    let tx_manager = env.transaction_manager();
    generate_node_input_for_role_group(
        runtime,
        networks.clone(),
        acco_key.public(),
        logger.clone(),
        tx_manager.clone(),
        keystore.clone(),
    )
}

pub async fn wait_for_connection_to_bootnodes<KBE: KeystoreBackend, Env: GadgetEnvironment>(
    config: &ShellConfig<KBE, Env>,
    handles: &HashMap<String, GossipHandle>,
    logger: &DebugLogger,
) -> color_eyre::Result<()> {
    let n_required = config.bootnodes.len();
    let n_networks = handles.len();
    logger.debug(format!(
        "Waiting for {n_required} peers to show up across {n_networks} networks"
    ));

    let mut tasks = gadget_io::tokio::task::JoinSet::new();

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
            gadget_io::tokio::time::sleep(Duration::from_millis(1000)).await;
        }
    };

    for handle in handles.values() {
        tasks.spawn(wait_for_peers(handle.clone(), n_required, logger.clone()));
    }
    // Wait for all tasks to finish
    while tasks.join_next().await.is_some() {}

    Ok(())
}
