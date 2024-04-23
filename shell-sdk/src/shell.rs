use std::collections::HashMap;
use std::time::Duration;
use std::{hash::Hash, sync::Arc};

use crate::{
    keystore::KeystoreContainer,
    tangle::{TangleConfig, TangleRuntime},
};
use color_eyre::eyre::OptionExt;
use futures::stream::FuturesOrdered;
use futures::TryStreamExt;
use gadget_common::keystore::KeystoreBackend;
use gadget_common::{
    client::{PairSigner, SubxtPalletSubmitter},
    config::{DebugLogger, PrometheusConfig},
    full_protocol::NodeInput,
    keystore::{ECDSAKeyStore, InMemoryBackend},
};
use sp_core::{ecdsa, ed25519, keccak_256, sr25519, ByteArray, Pair};
use sp_keystore::Keystore;
use tangle_runtime::api::runtime_types::tangle_primitives::roles::tss::ThresholdSignatureRoleType;
use tangle_runtime::api::runtime_types::tangle_primitives::roles::RoleType;
use tangle_subxt::subxt;
use tangle_subxt::tangle_testnet_runtime as tangle_runtime;
use tokio::task::JoinHandle;

use crate::config::ShellConfig;
use crate::network::gossip::GossipHandle;
use crate::tangle::crypto;
use itertools::Itertools;

/// The version of the shell-sdk
pub const AGENT_VERSION: &str = "tangle/gadget-shell-sdk/1.0.0";
pub const CLIENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Start the shell-sdk and run it forever
#[tracing::instrument(skip(config))]
pub async fn run_forever(config: ShellConfig) -> color_eyre::Result<()> {
    let (role_key, acco_key) = load_keys_from_keystore(&config.keystore)?;
    let network_key = ed25519::Pair::from_seed(&config.node_key);
    let logger = DebugLogger::default();
    let wrapped_keystore = ECDSAKeyStore::new(InMemoryBackend::new(), role_key.clone());

    let libp2p_key = libp2p::identity::Keypair::ed25519_from_bytes(network_key.to_raw_vec())
        .map_err(|e| color_eyre::eyre::eyre!("Failed to create libp2p keypair: {e}"))?;

    // Create a network for each subprotocol in the protocol (e.g., keygen, signing, refresh, rotate, = 4 total subprotocols = n_protocols)
    let network_ids = (0..config.n_protocols)
        .into_iter()
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

    let protocols = start_required_protocols(
        &config.subxt,
        networks,
        acco_key,
        logger,
        wrapped_keystore,
        config.role_types.clone(),
    );

    let ctrl_c = tokio::signal::ctrl_c();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl-C, shutting down");
        }
        res = protocols => {
            if let Err(e) = res {
                tracing::error!(error = ?e, "Protocols watcher task unexpectedly shutdown");
            }
        }

        _ = network_task => {
            tracing::error!("Network task unexpectedly shutdown")
        }
    }

    Ok(())
}

pub fn start_protocol_by_role<KBE>(
    role: RoleType,
    runtime: TangleRuntime,
    networks: HashMap<String, GossipHandle>,
    account_id: sr25519::Public,
    logger: DebugLogger,
    pallet_tx: Arc<SubxtPalletSubmitter<TangleConfig, PairSigner<TangleConfig>>>,
    keystore: ECDSAKeyStore<KBE>,
) -> color_eyre::Result<JoinHandle<()>>
where
    KBE: KeystoreBackend,
{
    use RoleType::*;
    use ThresholdSignatureRoleType::*;
    let networks = networks
        .into_iter()
        .sorted_by_key(|r| r.0.clone())
        .map(|r| r.1)
        .collect::<Vec<_>>();
    let clients = (0..networks.len())
        .map(|_| TangleRuntime::new(runtime.client()))
        .collect::<Vec<_>>();
    // TODO: Cleanup loagic below
    let handle = match role {
        Tss(DfnsCGGMP21Stark) | Tss(DfnsCGGMP21Secp256r1) | Tss(DfnsCGGMP21Secp256k1) => {
            tokio::spawn(dfns_cggmp21_protocol::setup_node(NodeInput {
                clients,
                account_id,
                logger,
                pallet_tx,
                keystore,
                node_index: 0,
                additional_params: (),
                prometheus_config: PrometheusConfig::Disabled,
                networks,
            }))
        }

        Tss(ZcashFrostEd25519)
        | Tss(ZcashFrostEd448)
        | Tss(ZcashFrostP256)
        | Tss(ZcashFrostP384)
        | Tss(ZcashFrostSecp256k1)
        | Tss(ZcashFrostRistretto255) => {
            tokio::spawn(zcash_frost_protocol::setup_node(NodeInput {
                clients,
                account_id,
                logger,
                pallet_tx,
                keystore,
                node_index: 0,
                additional_params: (),
                prometheus_config: PrometheusConfig::Disabled,
                networks,
            }))
        }
        Tss(GennaroDKGBls381) => tokio::spawn(threshold_bls_protocol::setup_node(NodeInput {
            clients,
            account_id,
            logger,
            pallet_tx,
            keystore,
            node_index: 0,
            additional_params: (),
            prometheus_config: PrometheusConfig::Disabled,
            networks,
        })),
        Tss(WstsV2) => tokio::spawn(wsts_protocol::setup_node(NodeInput {
            clients,
            account_id,
            logger,
            pallet_tx,
            keystore,
            node_index: 0,
            additional_params: (),
            prometheus_config: PrometheusConfig::Disabled,
            networks,
        })),
        ZkSaaS(_) => tokio::task::spawn(zk_saas_protocol::setup_node(NodeInput {
            clients,
            account_id,
            logger,
            pallet_tx,
            keystore,
            node_index: 0,
            additional_params: (),
            prometheus_config: PrometheusConfig::Disabled,
            networks,
        })),
        LightClientRelaying => {
            return Err(color_eyre::eyre::eyre!(
                "LightClientRelaying is not supported by the shell-sdk",
            ))
        }
        _ => {
            return Err(color_eyre::eyre::eyre!(
                "Role {:?} is not supported by the shell-sdk",
                role
            ))
        }
    };

    Ok(handle)
}

pub async fn start_required_protocols<KBE>(
    subxt_config: &crate::config::SubxtConfig,
    networks: HashMap<String, GossipHandle>,
    acco_key: sr25519::Pair,
    logger: DebugLogger,
    keystore: ECDSAKeyStore<KBE>,
    roles: Vec<RoleType>,
) -> color_eyre::Result<()>
where
    KBE: KeystoreBackend,
{
    let subxt_client =
        subxt::OnlineClient::<subxt::PolkadotConfig>::from_url(&subxt_config.endpoint).await?;

    let pair_signer = PairSigner::new(acco_key.clone());
    let pallet_tx_submitter =
        SubxtPalletSubmitter::with_client(subxt_client.clone(), pair_signer, logger.clone());
    let pallet_tx = Arc::new(pallet_tx_submitter);
    let runtime = TangleRuntime::new(subxt_client);
    logger.trace(format!("Got roles: {roles:?}"));
    let mut futures = FuturesOrdered::new();
    for role in roles {
        let handle = start_protocol_by_role(
            role,
            TangleRuntime::new(runtime.client()),
            networks.clone(),
            acco_key.public(),
            logger.clone(),
            pallet_tx.clone(),
            keystore.clone(),
        )?;

        futures.push_back(handle);
    }

    let result = futures.try_collect::<Vec<_>>().await;

    Err(color_eyre::Report::msg(format!(
        "Protocol stream ended unexpectedly: {result:?}"
    )))
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
    handles: &HashMap<String, GossipHandle>,
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

    for handle in handles.values() {
        tasks.spawn(wait_for_peers(handle.clone(), n_required, logger.clone()));
    }
    // Wait for all tasks to finish
    while tasks.join_next().await.is_some() {}

    Ok(())
}

#[derive(Eq, Clone)]
struct HashedRoleTypeWrapper(RoleType);

impl std::fmt::Debug for HashedRoleTypeWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl PartialEq for HashedRoleTypeWrapper {
    fn eq(&self, other: &Self) -> bool {
        use std::hash::Hasher;
        let mut hasher = std::hash::DefaultHasher::new();
        Self::hash(self, &mut hasher);
        let lhs = hasher.finish();
        let mut hasher = std::hash::DefaultHasher::new();
        Self::hash(other, &mut hasher);
        let rhs = hasher.finish();
        lhs == rhs
    }
}

impl Hash for HashedRoleTypeWrapper {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        use RoleType::*;
        use ThresholdSignatureRoleType::*;
        match self.0 {
            Tss(DfnsCGGMP21Stark) | Tss(DfnsCGGMP21Secp256r1) | Tss(DfnsCGGMP21Secp256k1) => {
                "DfnsCGGMP21".hash(state)
            }
            Tss(ZcashFrostEd25519)
            | Tss(ZcashFrostEd448)
            | Tss(ZcashFrostP256)
            | Tss(ZcashFrostP384)
            | Tss(ZcashFrostSecp256k1)
            | Tss(ZcashFrostRistretto255) => "ZcashFrost".hash(state),
            Tss(WstsV2) => "WstsV2".hash(state),
            Tss(SilentShardDKLS23Secp256k1) => "SilentShardDKLS23Secp256k1".hash(state),
            Tss(GennaroDKGBls381) => "GennaroDKGBls381".hash(state),
            ZkSaaS(_) => "ZkSaaS".hash(state),
            LightClientRelaying => "LightClientRelaying".hash(state),
        }
    }
}
