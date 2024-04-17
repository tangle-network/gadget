use std::collections::{HashMap, HashSet};
// use std::time::Duration;
use std::{hash::Hash, sync::Arc};

use crate::{
    // keystore::KeystoreContainer,
    // tangle::{TangleConfig, TangleRuntime, crypto},
    network::gossip::GossipHandle,
    config::ShellConfig,
};
use color_eyre::eyre::OptionExt;
use gadget_common::keystore::KeystoreBackend;
use gadget_common::{
    client::{PairSigner, SubxtPalletSubmitter},
    config::{ClientWithApi, DebugLogger, PrometheusConfig},
    full_protocol::NodeInput,
    keystore::{ECDSAKeyStore, InMemoryBackend},
};
// use gadget_core::gadget::substrate::Client;
use sp_core::{ecdsa, ed25519, sr25519, ByteArray, Pair};
// use sp_keystore::Keystore;
use gadget_io::{KeystoreConfig, SubstrateKeystore};
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::roles::tss::ThresholdSignatureRoleType;
use tangle_subxt::{
    subxt, tangle_testnet_runtime::api::runtime_types::tangle_primitives::roles::RoleType,
};

use crate::log;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures;
use gadget_io::*;
use gadget_io::into_js_error as into_js_error;


// use dfns_cggmp21_protocol::constants::{
//     DFNS_CGGMP21_KEYGEN_PROTOCOL_NAME, DFNS_CGGMP21_KEYREFRESH_PROTOCOL_NAME,
//     DFNS_CGGMP21_KEYROTATE_PROTOCOL_NAME, DFNS_CGGMP21_SIGNING_PROTOCOL_NAME,
// };
// use silent_shard_dkls23_ll_protocol::constants::{
//     SILENT_SHARED_DKLS23_KEYGEN_PROTOCOL_NAME, SILENT_SHARED_DKLS23_SIGNING_PROTOCOL_NAME,
// };
// use threshold_bls_protocol::constants::{
//     GENNARO_DKG_BLS_381_KEYGEN_PROTOCOL_NAME, GENNARO_DKG_BLS_381_SIGNING_PROTOCOL_NAME,
// };
// use zcash_frost_protocol::constants::{
//     ZCASH_FROST_KEYGEN_PROTOCOL_NAME, ZCASH_FROST_SIGNING_PROTOCOL_NAME,
// };

/// The version of the shell
pub const AGENT_VERSION: &str = "tangle/gadget-shell/1.0.0";
pub const CLIENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Start the shell and run it forever
#[tracing::instrument(skip(config))]
pub async fn run_forever(config: ShellConfig) -> Result<JsValue, JsValue> {
    log(&format!("Shell Config: {:?}", config));
    let (role_key, acco_key) = load_keys_from_keystore(config.keystore.clone()).map_err(|e| js_sys::Error::new(&e.to_string()))?;//.map_err(|err| Err(format!("{:?}", err))).map_err(into_js_error)?;

    let network_key = ed25519::Pair::from_seed(&config.node_key);
    log(&format!("NETWORK (ED25519) KEY PAIR?: {:?}", network_key.to_raw_vec()));

    let logger = DebugLogger::default();
    let wrapped_keystore = ECDSAKeyStore::new(InMemoryBackend::new(), role_key.clone());

    let libp2p_key = libp2p::identity::Keypair::ed25519_from_bytes(network_key.to_raw_vec())
        .map_err(|e| js_sys::Error::new(&e.to_string()))?;
    log(&format!("LIBP2P KEY PAIR?: {:?}", libp2p_key));

    let (networks, network_task) = crate::network::setup::setup_libp2p_network(
        libp2p_key,
        &config,
        logger.clone(),
        vec![
            // // dfns-cggmp21
            // DFNS_CGGMP21_KEYGEN_PROTOCOL_NAME,
            // DFNS_CGGMP21_SIGNING_PROTOCOL_NAME,
            // DFNS_CGGMP21_KEYREFRESH_PROTOCOL_NAME,
            // DFNS_CGGMP21_KEYROTATE_PROTOCOL_NAME,
            // // zcash-frost
            // ZCASH_FROST_KEYGEN_PROTOCOL_NAME,
            // ZCASH_FROST_SIGNING_PROTOCOL_NAME,
            // // silent-shared-dkls23
            // SILENT_SHARED_DKLS23_KEYGEN_PROTOCOL_NAME,
            // SILENT_SHARED_DKLS23_SIGNING_PROTOCOL_NAME,
            // // gennaro-dkg-bls381
            // GENNARO_DKG_BLS_381_KEYGEN_PROTOCOL_NAME,
            // GENNARO_DKG_BLS_381_SIGNING_PROTOCOL_NAME,
        ],
        role_key,
    )
        .await
        .map_err(|e| js_sys::Error::new(&e.to_string()))?;
        // .map_err(|e| color_eyre::eyre::eyre!("Failed to setup network: {e}"))?;

    // logger.debug("Successfully initialized network, now waiting for bootnodes to connect ...");
    // wait_for_connection_to_bootnodes(&config, &networks, &logger).await?;

    // let protocols =
    //     start_required_protocols(&config.subxt, networks, acco_key, logger, wrapped_keystore);

    // let ctrl_c = tokio::signal::ctrl_c();
    //
    // tokio::select! {
    //     _ = ctrl_c => {
    //         tracing::info!("Received Ctrl-C, shutting down");
    //     }
    //     res = protocols => {
    //         if let Err(e) = res {
    //             tracing::error!(error = ?e, "Protocols watcher task unexpectedly shutdown");
    //         }
    //     }
    //
    //     _ = network_task => {
    //         tracing::error!("Network task unexpectedly shutdown")
    //     }
    // }
    // log(&format!("Shell Checkpoint with key {:?}!", libp2p_key));
    let result = serde_wasm_bindgen::to_value("success message").map_err(into_js_error)?;
    Ok(result)
}
//
// pub fn start_protocol_by_role<KBE>(
//     role: RoleType,
//     runtime: TangleRuntime,
//     networks: HashMap<&'static str, GossipHandle>,
//     account_id: sr25519::Public,
//     logger: DebugLogger,
//     pallet_tx: Arc<SubxtPalletSubmitter<TangleConfig, PairSigner<TangleConfig>>>,
//     keystore: ECDSAKeyStore<KBE>,
// ) -> color_eyre::Result<tokio::task::AbortHandle>
//     where
//         KBE: KeystoreBackend,
// {
//     use RoleType::*;
//     use ThresholdSignatureRoleType::*;
//     let handle = match role {
//         Tss(DfnsCGGMP21Stark) | Tss(DfnsCGGMP21Secp256r1) | Tss(DfnsCGGMP21Secp256k1) => {
//             tokio::spawn(dfns_cggmp21_protocol::setup_node(NodeInput {
//                 clients: vec![
//                     TangleRuntime::new(runtime.client()),
//                     TangleRuntime::new(runtime.client()),
//                     TangleRuntime::new(runtime.client()),
//                     TangleRuntime::new(runtime.client()),
//                 ],
//                 account_id,
//                 logger,
//                 pallet_tx,
//                 keystore,
//                 node_index: 0,
//                 additional_params: (),
//                 prometheus_config: PrometheusConfig::Disabled,
//                 networks: vec![
//                     networks
//                         .get(DFNS_CGGMP21_KEYGEN_PROTOCOL_NAME)
//                         .cloned()
//                         .unwrap(),
//                     networks
//                         .get(DFNS_CGGMP21_SIGNING_PROTOCOL_NAME)
//                         .cloned()
//                         .unwrap(),
//                     networks
//                         .get(DFNS_CGGMP21_KEYREFRESH_PROTOCOL_NAME)
//                         .cloned()
//                         .unwrap(),
//                     networks
//                         .get(DFNS_CGGMP21_KEYROTATE_PROTOCOL_NAME)
//                         .cloned()
//                         .unwrap(),
//                 ],
//             }))
//         }
//
//         Tss(ZcashFrostEd25519)
//         | Tss(ZcashFrostEd448)
//         | Tss(ZcashFrostP256)
//         | Tss(ZcashFrostP384)
//         | Tss(ZcashFrostSecp256k1)
//         | Tss(ZcashFrostRistretto255) => {
//             tokio::spawn(zcash_frost_protocol::setup_node(NodeInput {
//                 clients: vec![
//                     TangleRuntime::new(runtime.client()),
//                     TangleRuntime::new(runtime.client()),
//                 ],
//                 account_id,
//                 logger,
//                 pallet_tx,
//                 keystore,
//                 node_index: 0,
//                 additional_params: (),
//                 prometheus_config: PrometheusConfig::Disabled,
//                 networks: vec![
//                     networks
//                         .get(ZCASH_FROST_KEYGEN_PROTOCOL_NAME)
//                         .cloned()
//                         .unwrap(),
//                     networks
//                         .get(ZCASH_FROST_SIGNING_PROTOCOL_NAME)
//                         .cloned()
//                         .unwrap(),
//                 ],
//             }))
//         }
//
//         Tss(SilentShardDKLS23Secp256k1) => {
//             tokio::spawn(silent_shard_dkls23_ll_protocol::setup_node(NodeInput {
//                 clients: vec![
//                     TangleRuntime::new(runtime.client()),
//                     TangleRuntime::new(runtime.client()),
//                 ],
//                 account_id,
//                 logger,
//                 pallet_tx,
//                 keystore,
//                 node_index: 0,
//                 additional_params: (),
//                 prometheus_config: PrometheusConfig::Disabled,
//                 networks: vec![
//                     networks
//                         .get(SILENT_SHARED_DKLS23_KEYGEN_PROTOCOL_NAME)
//                         .cloned()
//                         .unwrap(),
//                     networks
//                         .get(SILENT_SHARED_DKLS23_SIGNING_PROTOCOL_NAME)
//                         .cloned()
//                         .unwrap(),
//                 ],
//             }))
//         }
//         Tss(GennaroDKGBls381) => tokio::spawn(threshold_bls_protocol::setup_node(NodeInput {
//             clients: vec![
//                 TangleRuntime::new(runtime.client()),
//                 TangleRuntime::new(runtime.client()),
//             ],
//             account_id,
//             logger,
//             pallet_tx,
//             keystore,
//             node_index: 0,
//             additional_params: (),
//             prometheus_config: PrometheusConfig::Disabled,
//             networks: vec![
//                 networks
//                     .get(GENNARO_DKG_BLS_381_KEYGEN_PROTOCOL_NAME)
//                     .cloned()
//                     .unwrap(),
//                 networks
//                     .get(GENNARO_DKG_BLS_381_SIGNING_PROTOCOL_NAME)
//                     .cloned()
//                     .unwrap(),
//             ],
//         })),
//         ZkSaaS(_) => {
//             return Err(color_eyre::eyre::eyre!(
//                 "ZkSaaS is not supported by the shell",
//             ))
//         }
//         LightClientRelaying => {
//             return Err(color_eyre::eyre::eyre!(
//                 "LightClientRelaying is not supported by the shell",
//             ))
//         }
//     };
//
//     Ok(handle.abort_handle())
// }
//
// pub async fn start_required_protocols<KBE>(
//     subxt_config: &crate::config::SubxtConfig,
//     networks: HashMap<&'static str, GossipHandle>,
//     acco_key: sr25519::Pair,
//     logger: DebugLogger,
//     keystore: ECDSAKeyStore<KBE>,
// ) -> color_eyre::Result<()>
//     where
//         KBE: KeystoreBackend,
// {
//     let sub_account_id = subxt::utils::AccountId32(acco_key.public().0);
//     // Create a loop that listens to new finality notifications,
//     // queries the chain for the current restaking roles,
//     // and then starts the required protocols based on the roles.
//     let mut current_roles = Vec::new();
//     let mut running_protocols = HashMap::<HashedRoleTypeWrapper, tokio::task::AbortHandle>::new();
//
//     let subxt_client =
//         subxt::OnlineClient::<subxt::PolkadotConfig>::from_url(&subxt_config.endpoint).await?;
//
//     let pair_signer = PairSigner::new(acco_key.clone());
//     let pallet_tx_submitter =
//         SubxtPalletSubmitter::with_client(subxt_client.clone(), pair_signer, logger.clone());
//     let pallet_tx = Arc::new(pallet_tx_submitter);
//     let runtime = TangleRuntime::new(subxt_client);
//     while let Some(notification) = runtime.get_next_finality_notification().await {
//         let roles = runtime
//             .query_restaker_roles(notification.hash, sub_account_id.clone())
//             .await?;
//         logger.trace(format!("Got roles: {roles:?}"));
//         if roles == current_roles {
//             logger.trace("Roles have not changed, skipping");
//             continue;
//         }
//         let diff = vec_diff(
//             current_roles.iter().cloned().map(HashedRoleTypeWrapper),
//             roles.iter().cloned().map(HashedRoleTypeWrapper),
//         );
//         if diff.is_empty() {
//             logger.trace("No roles diff, skipping");
//             continue;
//         }
//         logger.trace(format!("Roles diff: {diff:?}"));
//         for d in diff {
//             match d {
//                 Diff::Added(role) => {
//                     logger.debug(format!("Trying to start protocol for role {:?}", role.0));
//                     let handle = start_protocol_by_role(
//                         role.0.clone(),
//                         TangleRuntime::new(runtime.client()),
//                         networks.clone(),
//                         acco_key.public(),
//                         logger.clone(),
//                         pallet_tx.clone(),
//                         keystore.clone(),
//                     )?;
//                     running_protocols.insert(role, handle);
//                 }
//                 Diff::Removed(role) => {
//                     logger.debug(format!("Trying to stop protocol for role {:?}", role.0));
//                     let maybe_handle = running_protocols.remove(&role);
//                     if let Some(handle) = maybe_handle {
//                         handle.abort();
//                         logger.warn(format!(
//                             "Aborted protocol for role {:?}. Reason: Role Removed from profile",
//                             role.0
//                         ));
//                     }
//                 }
//             }
//         }
//         current_roles = roles;
//     }
//     Ok(())
// }

pub fn load_keys_from_keystore<T: SubstrateKeystore> (
    keystore_config: T,
) -> color_eyre::Result<(ecdsa::Pair, sr25519::Pair)> {
    Ok((keystore_config.ecdsa_key()?, keystore_config.sr25519_key()?))
}

// pub async fn wait_for_connection_to_bootnodes(
//     config: &ShellConfig,
//     handles: &HashMap<&'static str, GossipHandle>,
//     logger: &DebugLogger,
// ) -> color_eyre::Result<()> {
//     let n_required = config.bootnodes.len();
//     let n_networks = handles.len();
//     logger.debug(format!(
//         "Waiting for {n_required} peers to show up across {n_networks} networks"
//     ));
//
//     let mut tasks = tokio::task::JoinSet::new();
//
//     // For each network, we start a task that checks if we have enough peers connected
//     // and then we wait for all of them to finish.
//
//     let wait_for_peers = |handle: GossipHandle, n_required, logger: DebugLogger| async move {
//         'inner: loop {
//             let n_connected = handle.connected_peers();
//             if n_connected >= n_required {
//                 break 'inner;
//             }
//             let topic = handle.topic();
//             logger.debug(format!("`{topic}`: We currently have {n_connected}/{n_required} peers connected to network"));
//             tokio::time::sleep(Duration::from_millis(1000)).await;
//         }
//     };
//
//     for handle in handles.values() {
//         tasks.spawn(wait_for_peers(handle.clone(), n_required, logger.clone()));
//     }
//     // Wait for all tasks to finish
//     while (tasks.join_next().await).is_some() {}
//
//     Ok(())
// }
//
// #[derive(Debug, PartialEq, Eq, Clone)]
// enum Diff<T> {
//     Added(T),
//     Removed(T),
// }
//
// #[derive(Eq, Clone)]
// struct HashedRoleTypeWrapper(RoleType);
//
// impl std::fmt::Debug for HashedRoleTypeWrapper {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{:?}", self.0)
//     }
// }
//
// impl PartialEq for HashedRoleTypeWrapper {
//     fn eq(&self, other: &Self) -> bool {
//         use std::hash::Hasher;
//         let mut hasher = std::hash::DefaultHasher::new();
//         Self::hash(self, &mut hasher);
//         let lhs = hasher.finish();
//         let mut hasher = std::hash::DefaultHasher::new();
//         Self::hash(other, &mut hasher);
//         let rhs = hasher.finish();
//         lhs == rhs
//     }
// }
//
// impl Hash for HashedRoleTypeWrapper {
//     fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
//         use RoleType::*;
//         use ThresholdSignatureRoleType::*;
//         match self.0 {
//             Tss(DfnsCGGMP21Stark) | Tss(DfnsCGGMP21Secp256r1) | Tss(DfnsCGGMP21Secp256k1) => {
//                 "DfnsCGGMP21".hash(state)
//             }
//             Tss(ZcashFrostEd25519)
//             | Tss(ZcashFrostEd448)
//             | Tss(ZcashFrostP256)
//             | Tss(ZcashFrostP384)
//             | Tss(ZcashFrostSecp256k1)
//             | Tss(ZcashFrostRistretto255) => "ZcashFrost".hash(state),
//
//             Tss(SilentShardDKLS23Secp256k1) => "SilentShardDKLS23Secp256k1".hash(state),
//             Tss(GennaroDKGBls381) => "GennaroDKGBls381".hash(state),
//             ZkSaaS(_) => "ZkSaaS".hash(state),
//             LightClientRelaying => "LightClientRelaying".hash(state),
//         }
//     }
// }
//
// fn vec_diff<T: PartialEq + Eq + Hash + Clone, I: Iterator<Item = T>>(a: I, b: I) -> Vec<Diff<T>> {
//     let a_set = HashSet::<T, std::hash::RandomState>::from_iter(a);
//     let b_set = HashSet::from_iter(b);
//     // find the elements that are in a but not in b (removed)
//     let removed = a_set
//         .difference(&b_set)
//         .cloned()
//         .map(Diff::Removed)
//         .collect::<Vec<_>>();
//     // find the elements that are in b but not in a (added)
//     let added = b_set
//         .difference(&a_set)
//         .cloned()
//         .map(Diff::Added)
//         .collect::<Vec<_>>();
//     [removed, added].concat()
// }
//
// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     #[test]
//     fn diff() {
//         let a = [1, 2, 3, 4];
//         let b = [2, 3, 4, 5];
//         let diff = vec_diff(a.iter().cloned(), b.iter().cloned());
//         assert_eq!(diff, vec![Diff::Removed(1), Diff::Added(5)]);
//     }
// }
