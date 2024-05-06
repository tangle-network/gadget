use color_eyre::eyre::WrapErr;
use color_eyre::*;
use console_error_panic_hook;
use gadget_io::tokio::sync::{Mutex, RwLock};
use gadget_io::{into_js_error, log, KeystoreConfig, Opt, SupportedChains, TomlConfig};
use sp_core::{ecdsa, ed25519, sr25519, ByteArray, Pair};

use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::panic;
use std::sync::atomic::AtomicU32;
use std::{error::Error, fmt::Display, net::IpAddr, path::PathBuf, str::FromStr};

use tangle_subxt::subxt;
use tangle_subxt::tangle_testnet_runtime as tangle_runtime;
use tangle_runtime::api::runtime_types::tangle_primitives::roles::tss::ThresholdSignatureRoleType;
use tangle_runtime::api::runtime_types::tangle_primitives::roles::RoleType;

use structopt::StructOpt;
use url::Url;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures;

use crate::config;
use crate::config::ShellConfig;
use crate::shell;
use crate::shell::{HashedRoleTypeWrapper, vec_diff, Diff};
use crate::tangle::{TangleRuntime, TangleConfig};

use gadget_common::prelude::*;
use gadget_common::ExecutableJob;
use gadget_core::gadget::substrate::Client;
use tsify::Tsify;

use futures::{select, FutureExt, SinkExt};
use futures_timer::Delay;
use matchbox_socket::{PeerId, PeerState, WebRtcSocket};
use std::time::Duration;

// use wasm_bindgen_test::wasm_bindgen_test;
use log::info;
use gadget_io::tokio::sync::oneshot::Sender;
use zcash_frost_protocol::constants::{ZCASH_FROST_KEYGEN_PROTOCOL_NAME, ZCASH_FROST_SIGNING_PROTOCOL_NAME};

#[derive(Clone)]
pub struct MatchboxHandle {
    pub network: &'static str,
    pub connected_peers: Arc<AtomicU32>,
    pub tx_to_outbound: gadget_io::tokio::sync::mpsc::UnboundedSender<IntraNodePayload>,
    pub rx_from_inbound: Arc<Mutex<UnboundedReceiver<Vec<u8>>>>,
    pub ecdsa_peer_id_to_matchbox_id: Arc<RwLock<HashMap<ecdsa::Public, matchbox_socket::PeerId>>>,
}

impl MatchboxHandle {
    pub fn connected_peers(&self) -> usize {
        self.connected_peers
            .load(std::sync::atomic::Ordering::Relaxed) as usize
    }

    pub fn topic(&self) -> &str {
        self.network.clone()
    }
}

pub struct IntraNodePayload {
    topic: String,
    payload: MatchboxGossipOrRequestResponse,
    message_type: MessageType,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum MatchboxGossipOrRequestResponse {
    Gossip(MatchboxMessage),
    Request(MyBehaviourRequest),
    Response(MyBehaviourResponse),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MatchboxMessage {
    pub topic: String,
    pub raw_payload: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum MyBehaviourRequest {
    Handshake {
        ecdsa_public_key: ecdsa::Public,
        signature: ecdsa::Signature,
    },
    Message {
        topic: String,
        raw_payload: Vec<u8>,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum MyBehaviourResponse {
    Handshaked {
        ecdsa_public_key: ecdsa::Public,
        signature: ecdsa::Signature,
    },
    MessageHandled,
}

enum MessageType {
    Broadcast,
    P2P(PeerId),
}

#[async_trait]
impl Network for MatchboxHandle {
    async fn next_message(&self) -> Option<<WorkManager as WorkManagerInterface>::ProtocolMessage> {
        let mut lock = self
            .rx_from_inbound
            .try_lock()
            .expect("There should be only a single caller for `next_message`");

        let message = lock.recv().await?;
        match bincode::deserialize(&message) {
            Ok(message) => Some(message),
            Err(e) => {
                // self.logger
                //     .error(format!("Failed to deserialize message: {e}"));
                drop(lock);
                self.next_message().await
            }
        }
    }

    async fn send_message(
        &self,
        message: <WorkManager as WorkManagerInterface>::ProtocolMessage,
    ) -> Result<(), gadget_common::Error> {
        let message_type = if let Some(to) = message.to_network_id {
            let matchbox_id = self
                .ecdsa_peer_id_to_matchbox_id
                .read()
                .await
                .get(&to)
                .cloned()
                .ok_or_else(|| gadget_common::Error::NetworkError {
                    err: format!(
                        "No Matchbox ID found for ecdsa public key: {to:?}. No handshake happened?"
                    ),
                })?;

            MessageType::P2P(matchbox_id)
        } else {
            MessageType::Broadcast
        };

        let payload_inner = match message_type {
            MessageType::Broadcast => MatchboxGossipOrRequestResponse::Gossip(MatchboxMessage {
                topic: self.topic().to_string(),
                raw_payload: bincode::serialize(&message).expect("Should serialize"),
            }),
            MessageType::P2P(_) => MatchboxGossipOrRequestResponse::Request(MyBehaviourRequest::Message {
                topic: self.topic().to_string(),
                raw_payload: bincode::serialize(&message).expect("Should serialize"),
            }),
        };

        let payload = IntraNodePayload {
            topic: self.topic().to_string().clone(),
            payload: payload_inner,
            message_type,
        };

        self.tx_to_outbound
            .send(payload)
            .map_err(|e| gadget_common::Error::NetworkError {
                err: format!("Failed to send intra-node payload: {e}"),
            })
    }
}

#[wasm_bindgen]
#[no_mangle]
pub async fn run_web_shell(options: Opt, keys: Vec<String>) -> Result<JsValue, JsValue> {
    // web_init();
    // color_eyre::install().map_err(|e| js_sys::Error::new(&e.to_string()))?;
    // let opt = Opt::from_args();
    // setup_logger(&opt, "gadget_shell")?;
    // log(&log_rust(&format!("Hello from web_main in Rust!")));
    log(&format!("Options: {:?}", options));

    let Opt {
        options,
        verbose,
        pretty,
        config,
    } = options;

    log(&format!("TomlConfig: {:?}", options));

    let TomlConfig {
        bind_ip,
        bind_port,
        url,
        bootnodes,
        node_key,
        base_path,
        keystore_password,
        chain,
    } = options;

    let endpoint = Url::parse(&url).map_err(into_js_error)?;
    log(&format!("Endpoint: {:?}", endpoint));

    let bind_ip = IpAddr::from_str(&bind_ip).map_err(into_js_error)?;
    log(&format!("Bind IP: {:?}", bind_ip));

    let bootnodes: Vec<Multiaddr> = bootnodes
        .iter()
        .map(|s| Multiaddr::from_str(&s))
        .filter_map(|x| x.ok())
        .collect();
    log(&format!("Bootnodes: {:?}", bootnodes));

    let node_key: [u8; 32] = if let Some(node_key) = node_key {
        hex::decode(&node_key)
    } else {
        hex::decode("0000000000000000000000000000000000000000000000000000000000000001")
        // TODO: Should generate
    }
    .map_err(into_js_error)?
    .as_slice()
    .try_into()
    .map_err(into_js_error)?;
    log(&format!("Node Key: {:?}", node_key));

    // let keys: [u8; 32] = hex::decode("0000000000000000000000000000000000000000000000000000000000000001").map_err(into_js_error)?.as_slice().try_into().map_err(into_js_error)?;
    //
    // let node_key: [u8; 32] = hex::decode("0000000000000000000000000000000000000000000000000000000000000001")
    //     .map_err(into_js_error)?
    //     .as_slice()
    //     .try_into()
    //     .map_err(into_js_error)?;

    shell::run_forever(config::ShellConfig {
        keystore: KeystoreConfig::InMemory { keystore: keys }, //"0000000000000000000000000000000000000000000000000000000000000001".to_string() },
        subxt: config::SubxtConfig { endpoint },
        base_path,
        bind_ip,
        bind_port,
        bootnodes,
        node_key,
    })
    .await
    .map_err(|e| js_sys::Error::new(&e.to_string()))?;
    let result = serde_wasm_bindgen::to_value("success message").map_err(into_js_error)?;
    Ok(result)
}

pub async fn setup_matchbox_network(
    // config: &ShellConfig,
    // logger: DebugLogger,
    networks: Vec<&'static str>,
    // role_key: ecdsa::Pair,
) -> Result<HashMap<&'static str, MatchboxHandle>, Box<dyn Error>> {
    log(&format!("SETUP MATCHBOX NETWORK"));

    let (mut socket, loop_fut) = WebRtcSocket::new_reliable("ws://localhost:3536/"); // Signaling Server Address

    // Create Channels for Sending and Receiving from Worker
    // Subscribe to all networks
    let mut inbound_mapping = Vec::new();
    let (tx_to_outbound, mut rx_to_outbound) =
        gadget_io::tokio::sync::mpsc::unbounded_channel::<IntraNodePayload>();
    let ecdsa_peer_id_to_matchbox_id = Arc::new(RwLock::new(HashMap::new()));
    let mut handles_ret = HashMap::with_capacity(networks.len());
    for network in networks {
        log(&format!("MATCHBOX NETWORK LOOP ITERATION: {:?}", network));
        let (inbound_tx, inbound_rx) = gadget_io::tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
        let connected_peers = Arc::new(AtomicU32::new(0));
        inbound_mapping.push((network.clone(), inbound_tx, connected_peers.clone()));
        handles_ret.insert(
            network,
            MatchboxHandle {
                network,
                connected_peers,
                tx_to_outbound: tx_to_outbound.clone(),
                rx_from_inbound: Arc::new(Mutex::new(inbound_rx)),
                ecdsa_peer_id_to_matchbox_id: ecdsa_peer_id_to_matchbox_id.clone(),
            },
        );
    }

    log(&format!("ALL NETWORKS READY"));

    let worker = async move {
        log(&format!("WORKER ASYNC MOVE ENTERED"));

        let span = tracing::debug_span!("network_worker");
        let _enter = span.enter();
        // let service = NetworkServiceWithoutSwarm {
        //     logger: &logger,
        //     inbound_mapping: &inbound_mapping,
        //     ecdsa_peer_id_to_libp2p_id,
        //     role_key: &role_key,
        //     span: tracing::debug_span!(parent: &span, "network_service"),
        // };
        // loop {
        //     select! {
        //         // Setup outbound channel
        //         Some(msg) = rx_to_outbound.recv() => {
        //             service.with_swarm(&mut swarm).handle_intra_node_payload(msg);
        //         }
        //         event = swarm.select_next_some() => {
        //             service.with_swarm(&mut swarm).handle_swarm_event(event).await;
        //         }
        //     }
        // }

        let loop_fut = loop_fut.fuse();
        futures::pin_mut!(loop_fut);
        let timeout = Delay::new(Duration::from_millis(100));
        futures::pin_mut!(timeout);
        // Listening Loop
        loop {
            log(&format!("MATCHBOX WORK LOOP ITERATION"));
            // When a new peer is found through relay
            for (peer, state) in socket.update_peers() {
                match state {
                    PeerState::Connected => {
                        let packet = "Message for Peer Two..."
                            .as_bytes()
                            .to_vec()
                            .into_boxed_slice();
                        socket.send(packet, peer);
                    }
                    PeerState::Disconnected => {
                        socket.close();
                    }
                }
            }
            // When a packet is received
            for (peer, packet) in socket.receive() {
                let message = String::from_utf8_lossy(&packet);
            }
            select! {
                // Loop every 100ms
                _ = (&mut timeout).fuse() => {
                    timeout.reset(Duration::from_millis(100));
                }
                // Break if the message loop ends via disconnect/closure
                _ = &mut loop_fut => {
                    break;
                }
            }
        }
    };
    wasm_bindgen_futures::spawn_local(worker);
    log(&format!(
        "SETUP MATCHBOX NETWORK EXITING AFTER SPAWNING WORKER"
    ));

    Ok(handles_ret)
}

pub async fn start_protocols_web<KBE>(
    subxt_config: &crate::config::SubxtConfig,
    networks: HashMap<&'static str, MatchboxHandle>,
    acco_key: sr25519::Pair,
    logger: DebugLogger,
    keystore: ECDSAKeyStore<KBE>,
) -> color_eyre::Result<()>
where
    KBE: KeystoreBackend,
{
    log(&format!("ENTERING START PROTOCOLS WEB"));
    let sub_account_id = subxt::utils::AccountId32(acco_key.public().0);
    // Create a loop that listens to new finality notifications,
    // queries the chain for the current restaking roles,
    // and then starts the required protocols based on the roles.
    log(&format!("INITIALIZING VEC AND MAP"));
    let mut current_roles = Vec::new();
    let mut running_protocols = HashMap::<HashedRoleTypeWrapper, gadget_io::tokio::task::AbortHandle>::new();

    log(&format!("CREATING SUBXT CLIENT"));
    let subxt_client =
        subxt::OnlineClient::<subxt::PolkadotConfig>::from_url(&subxt_config.endpoint).await?;

    log(&format!("CREATING PAIR SIGNER"));
    let pair_signer = PairSigner::new(acco_key.clone());
    log(&format!("CREATING PALLET TX SUBMITTER"));
    let pallet_tx_submitter =
        SubxtPalletSubmitter::with_client(subxt_client.clone(), pair_signer, logger.clone());
    log(&format!("CREATING PALLET TX"));
    let pallet_tx = Arc::new(pallet_tx_submitter);
    log(&format!("CREATING RUNTIME"));
    let runtime = TangleRuntime::new(subxt_client);
    log(&format!("WEB PROTOCOLS ABOUT TO ENTER NOTIFICATION LOOP"));
    while let Some(notification) = runtime.get_next_finality_notification().await {
        log(&format!("WEB PROTOCOLS NOTIFICATION LOOP ITERATION: {notification:?}"));
        let roles = runtime
            .query_restaker_roles(notification.hash, sub_account_id.clone())
            .await?;
        logger.trace(format!("Got roles: {roles:?}"));
        if roles == current_roles {
            logger.trace("Roles have not changed, skipping");
            continue;
        }
        let diff = vec_diff(
            current_roles.iter().cloned().map(HashedRoleTypeWrapper),
            roles.iter().cloned().map(HashedRoleTypeWrapper),
        );
        if diff.is_empty() {
            logger.trace("No roles diff, skipping");
            continue;
        }
        logger.trace(format!("Roles diff: {diff:?}"));
        for d in diff {
            match d {
                Diff::Added(role) => {
                    logger.debug(format!("Trying to start protocol for role {:?}", role.0));
                    let handle = start_web_protocol_by_role(
                        role.0.clone(),
                        TangleRuntime::new(runtime.client()),
                        networks.clone(),
                        acco_key.public(),
                        logger.clone(),
                        pallet_tx.clone(),
                        keystore.clone(),
                    )?;
                    running_protocols.insert(role, handle);
                }
                Diff::Removed(role) => {
                    logger.debug(format!("Trying to stop protocol for role {:?}", role.0));
                    let maybe_handle = running_protocols.remove(&role);
                    if let Some(handle) = maybe_handle {
                        handle.abort();
                        logger.warn(format!(
                            "Aborted protocol for role {:?}. Reason: Role Removed from profile",
                            role.0
                        ));
                    }
                }
            }
        }
        current_roles = roles;
    }
    log(&format!("WEB PROTOCOLS COMPLETED... EXITING START PROTOCOLS WEB"));
    Ok(())
}

pub fn start_web_protocol_by_role<KBE>(
    role: RoleType,
    runtime: TangleRuntime,
    networks: HashMap<&'static str, MatchboxHandle>,
    account_id: sr25519::Public,
    logger: DebugLogger,
    pallet_tx: Arc<SubxtPalletSubmitter<TangleConfig, PairSigner<TangleConfig>>>,
    keystore: ECDSAKeyStore<KBE>,
) -> color_eyre::Result<gadget_io::tokio::task::AbortHandle>
    where
        KBE: KeystoreBackend,
{
    use RoleType::*;
    use ThresholdSignatureRoleType::*;
    // let (abort_tx, abort_rx) = gadget_io::tokio::sync::oneshot::channel();

    let handle = match role {
        Tss(DfnsCGGMP21Stark) | Tss(DfnsCGGMP21Secp256r1) | Tss(DfnsCGGMP21Secp256k1) => {
            return Err(color_eyre::eyre::eyre!(
                "DfnsCGGMP21 is not supported by the web shell",
            ))
        }

        Tss(ZcashFrostEd25519)
        | Tss(ZcashFrostEd448)
        | Tss(ZcashFrostP256)
        | Tss(ZcashFrostP384)
        | Tss(ZcashFrostSecp256k1)
        | Tss(ZcashFrostRistretto255) => {
            log(&format!("Starting ZcashFrost protocol"));
            gadget_io::tokio::spawn(zcash_frost_protocol::setup_node(NodeInput {
                clients: vec![
                    TangleRuntime::new(runtime.client()),
                    TangleRuntime::new(runtime.client()),
                ],
                account_id,
                logger,
                pallet_tx,
                keystore,
                node_index: 0,
                additional_params: (),
                prometheus_config: PrometheusConfig::Disabled,
                networks: vec![
                    networks
                        .get(ZCASH_FROST_KEYGEN_PROTOCOL_NAME)
                        .cloned()
                        .unwrap(),
                    networks
                        .get(ZCASH_FROST_SIGNING_PROTOCOL_NAME)
                        .cloned()
                        .unwrap(),
                ],
            }))
        }
        Tss(GennaroDKGBls381) => {
            return Err(color_eyre::eyre::eyre!(
                "GennaroDKGBls381 is not supported by the web shell",
            ))
        }
        Tss(WstsV2) => {
            return Err(color_eyre::eyre::eyre!(
                "WstsV2 is not supported by the web shell",
            ))
        }
        ZkSaaS(_) => {
            return Err(color_eyre::eyre::eyre!(
                "ZkSaaS is not supported by the web shell",
            ))
        }
        LightClientRelaying => {
            return Err(color_eyre::eyre::eyre!(
                "LightClientRelaying is not supported by the web shell",
            ))
        }
        _ => {
            return Err(color_eyre::eyre::eyre!(
                "Role {:?} is not supported by the web shell",
                role
            ))
        }
    };
    Ok(handle.abort_handle())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test;
    use wasm_bindgen_test::wasm_bindgen_test_configure;
    wasm_bindgen_test_configure!(run_in_browser);

    use futures::{select, FutureExt};
    use futures_timer::Delay;
    use gadget_io::{log, Opt, SupportedChains, TomlConfig};
    use matchbox_socket::{MessageLoopFuture, PeerState, SingleChannel, WebRtcSocket};
    use std::time::Duration;

    #[wasm_bindgen_test]
    async fn test_web_main() {
        //-> Result<JsValue, JsValue> {
        let config = TomlConfig {
            bind_ip: "0.0.0.0".to_string().into(),
            bind_port: 30556,
            url: "ws://127.0.0.1:9944".to_string(),//"https://github.com/webb-tools/gadget".to_string(),
            bootnodes: vec![
                // "/ip4/127.0.0.1/udp/1234",
                "foo",
                // "/ip4/127.0.0.1/udp/1235",
                // "/ip4/127.0.0.1/udp/1236",
            ]
            .iter()
            .map(|&s| s.to_string())
            .collect(),
            node_key: Some(
                "0000000000000000000000000000000000000000000000000000000000000001".to_string(),
            ),
            base_path: "../../tangle/tmp/bob".into(),
            keystore_password: None,
            chain: SupportedChains::LocalTestnet,
        };

        let options = Opt {
            config: None,
            verbose: 0,
            pretty: false,
            options: config,
        };

        let keys = vec![
            "398f0c28f98885e046333d4a41c19cee4c37368a9832c6502f6cfd182e2aef89",
            "79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf",
            //"aaa88bec51838e7ec434d58f687b0c606b322f4e63ba9a9af2b6f1cb34f718be",
            // "0000000000000000000000000000000000000000000000000000000000000002",
        ]
        .iter()
        .map(|&s| s.to_string())
        .collect();
        run_web_shell(options, keys).await.unwrap();
    }

    // #[wasm_bindgen_test]
    // pub fn color_eyre_simple() {
    //     use color_eyre::eyre::WrapErr;
    //     use color_eyre::*;
    //
    //     install().expect("Failed to install color_eyre");
    //     let err_str = format!(
    //         "{:?}",
    //         Err::<(), Report>(eyre::eyre!("Base Error"))
    //             .note("A note")
    //             .suggestion("A suggestion")
    //             .wrap_err("A wrapped error")
    //             .unwrap_err()
    //     );
    //     // Print it out so if people run with `-- --nocapture`, they
    //     // can see the full message.
    //     println!("Error String is:\n\n{}", err_str);
    //     assert!(err_str.contains("A wrapped error"));
    //     assert!(err_str.contains("A suggestion"));
    //     assert!(err_str.contains("A note"));
    //     assert!(err_str.contains("Base Error"));
    // }

    /// Tests Browser-to-Browser connection using Matchbox WebRTC. Requires example matchbox_server
    /// running as signal server
    #[wasm_bindgen_test]
    fn test_matchbox_browser_to_browser() {
        console_error_panic_hook::set_once();
        console_log::init_with_level(log::Level::Debug).unwrap();

        let peer_one = async move {
            log(&format!("Peer One Beginning"));
            let (mut socket, loop_fut) = matchbox_socket_tester().await;
            let loop_fut = loop_fut.fuse();
            futures::pin_mut!(loop_fut);
            let timeout = Delay::new(Duration::from_millis(100));
            futures::pin_mut!(timeout);
            // Listening Loop
            loop {
                for (peer, state) in socket.update_peers() {
                    match state {
                        PeerState::Connected => {
                            log(&format!("Peer joined: {peer}"));
                            let packet = "Message for Peer Two..."
                                .as_bytes()
                                .to_vec()
                                .into_boxed_slice();
                            socket.send(packet, peer);
                        }
                        PeerState::Disconnected => {
                            log(&format!("Peer {peer} Disconnected, Closing Socket"));
                            socket.close();
                        }
                    }
                }
                for (peer, packet) in socket.receive() {
                    let message = String::from_utf8_lossy(&packet);
                    log(&format!("Message from {peer}: {message}"));
                }
                select! {
                    // Loop every 100ms
                    _ = (&mut timeout).fuse() => {
                        timeout.reset(Duration::from_millis(100));
                    }
                    // Break if the message loop ends via disconnect/closure
                    _ = &mut loop_fut => {
                        break;
                    }
                }
            }
        };

        let peer_two = async move {
            log(&format!("Peer Two Beginning"));
            let (mut socket, loop_fut) = matchbox_socket_tester().await;
            let loop_fut = loop_fut.fuse();
            futures::pin_mut!(loop_fut);
            let timeout = Delay::new(Duration::from_millis(100));
            futures::pin_mut!(timeout);
            // Listening Loop
            loop {
                for (peer, state) in socket.update_peers() {
                    match state {
                        _ => continue,
                    }
                }
                for (peer, packet) in socket.receive() {
                    let message = String::from_utf8_lossy(&packet);
                    log(&format!("Peer Two Received Message from {peer}: {message}"));
                    log(&format!("Peer Two Closing Socket"));
                    socket.close();
                }

                select! {
                    // Loop every 100ms
                    _ = (&mut timeout).fuse() => {
                        timeout.reset(Duration::from_millis(100));
                    }
                    // Break if the message loop ends via disconnect/closure
                    _ = &mut loop_fut => {
                        break;
                    }
                }
            }
        };

        wasm_bindgen_futures::spawn_local(peer_one);
        wasm_bindgen_futures::spawn_local(peer_two);
    }

    async fn matchbox_socket_tester() -> (WebRtcSocket<SingleChannel>, MessageLoopFuture) {
        log(&format!("Starting Matchbox Listener"));
        let (mut socket, loop_fut) = WebRtcSocket::new_reliable("ws://localhost:3536/"); // Signaling Server Address
        (socket, loop_fut)
    }

    // async fn async_main() {
    //     log(&format!("Hello from Matchbox Test!"));
    //     let (mut socket, loop_fut) = network::setup::matchbox_listener().await;
    //
    // }

    // #[wasm_bindgen_test]
    // fn test_prompt() {
    //     log(&format!("Input was {}!", webPrompt("Does this question appear?")));
    // }

    // #[wasm_bindgen_test]
    // async fn test_file_input() {
    //     log(&format!("File Success? {:?}!", get_from_js().await));
    //     //log(&format!("Input was {}!", webPrompt("Does this question appear?")));
    // }
}
