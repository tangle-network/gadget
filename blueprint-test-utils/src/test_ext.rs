// This file is part of Tangle.
// Copyright (C) 2022-2023 Webb Technologies Inc.
//
// Tangle is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Tangle is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Tangle.  If not, see <http://www.gnu.org/licenses/>.

use crate::PerTestNodeInput;
use crate::runtime_types::tangle_primitives::services::PriceTargets;
use blueprint_manager::executor::BlueprintManagerHandle;
use blueprint_manager::sdk::entry::SendFuture;
use blueprint_manager::sdk::setup::NodeInput;
use cargo_tangle::deploy::{Opts, PrivateKeySigner};
use gadget_sdk::clients::tangle::runtime::TangleRuntimeClient;
use gadget_sdk::logger::Logger;
use gadget_sdk::network::{Network, ParticipantInfo, ProtocolMessage};
use gadget_sdk::prometheus::PrometheusConfig;
use gadget_sdk::store::{ECDSAKeyStore, InMemoryBackend};
use gadget_sdk::tangle_subxt::subxt::utils::AccountId32;
use gadget_sdk::tangle_subxt::subxt::{OnlineClient, SubstrateConfig};
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::ApprovalPrefrence;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::services::calls::types::register::{Preferences, RegistrationArgs};
use gadget_sdk::TokioMutexExt;
use gadget_sdk::Error;
use libp2p::Multiaddr;
use sp_application_crypto::ecdsa;
use sp_core::{sr25519, Pair};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use std::collections::HashMap;
use std::future::Future;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use url::Url;

pub fn id_to_ecdsa_pair(id: u8) -> ecdsa::Pair {
    ecdsa::Pair::from_string(&format!("//Alice///{id}"), None).expect("static values are valid")
}

pub fn id_to_sr25519_pair(id: u8) -> sr25519::Pair {
    sr25519::Pair::from_string(&format!("//Alice///{id}"), None).expect("static values are valid")
}

pub fn id_to_public(id: u8) -> ecdsa::Public {
    id_to_ecdsa_pair(id).public()
}

pub fn id_to_sr25519_public(id: u8) -> sr25519::Public {
    id_to_sr25519_pair(id).public()
}

type PeersRx<Env> =
    Arc<HashMap<ecdsa::Public, gadget_io::tokio::sync::Mutex<UnboundedReceiver<Env>>>>;

pub struct MockNetwork {
    peers_tx: Arc<HashMap<ecdsa::Public, UnboundedSender<ProtocolMessage>>>,
    peers_rx: PeersRx<ProtocolMessage>,
    my_id: ecdsa::Public,
}

impl Clone for MockNetwork {
    fn clone(&self) -> Self {
        Self {
            peers_tx: self.peers_tx.clone(),
            peers_rx: self.peers_rx.clone(),
            my_id: self.my_id,
        }
    }
}

impl MockNetwork {
    pub fn setup(ids: &Vec<ecdsa::Public>) -> Vec<Self> {
        let mut peers_tx = HashMap::new();
        let mut peers_rx = HashMap::new();
        let mut networks = Vec::new();

        for id in ids {
            let (tx, rx) = gadget_io::tokio::sync::mpsc::unbounded_channel();
            peers_tx.insert(*id, tx);
            peers_rx.insert(*id, gadget_io::tokio::sync::Mutex::new(rx));
        }

        let peers_tx = Arc::new(peers_tx);
        let peers_rx = Arc::new(peers_rx);

        for id in ids {
            let network = Self {
                peers_tx: peers_tx.clone(),
                peers_rx: peers_rx.clone(),
                my_id: *id,
            };
            networks.push(network);
        }

        networks
    }
}

#[async_trait::async_trait]
impl Network for MockNetwork {
    async fn next_message(&self) -> Option<ProtocolMessage> {
        self.peers_rx
            .get(&self.my_id)?
            .lock_timeout(Duration::from_millis(500))
            .await
            .recv()
            .await
    }

    async fn send_message(&self, message: ProtocolMessage) -> Result<(), Error> {
        let _check_message_has_ids = message.sender.ecdsa_key.ok_or(Error::MissingNetworkId)?;
        if let Some(ParticipantInfo {
            ecdsa_key: Some(peer_id),
            ..
        }) = message.recipient
        {
            let tx = self
                .peers_tx
                .get(&peer_id)
                .ok_or(Error::PeerNotFound { id: peer_id })?;
            tx.send(message).map_err(|err| Error::NetworkError {
                reason: err.to_string(),
            })?;
        } else {
            // Broadcast to everyone except ourself
            for (peer_id, tx) in self.peers_tx.iter() {
                if peer_id != &self.my_id {
                    tx.send(message.clone())
                        .map_err(|err| Error::NetworkError {
                            reason: err.to_string(),
                        })?;
                }
            }
        }
        Ok(())
    }
}

const LOCAL_BIND_ADDR: &str = "127.0.0.1";
const LOCAL_TANGLE_NODE: &str = "ws://127.0.0.1:9944";

/// - `N`: number of nodes
/// - `K`: Number of networks accessible per node (should be equal to the number of services in a given blueprint)
/// - `D`: Any data that you want to pass to pass with NodeInput.
/// - `F`: A function that generates a service's execution via a series of shells. Each shell executes a subset of the service,
///        as each service may have a set of operations that are executed in parallel, sequentially, or concurrently.
pub async fn new_test_ext_blueprint_manager<
    const N: usize,
    const K: usize,
    D: Send + Clone + 'static,
    F: Fn(PerTestNodeInput<D>) -> Fut,
    Fut: SendFuture<'static, BlueprintManagerHandle>,
>(
    additional_params: D,
    mut opts: Opts,
    f: F,
) -> LocalhostTestExt {
    const NAME_IDS: [&str; 3] = ["alice", "bob", "charlie"];

    assert!(N < 4, "Only up to 3 nodes are supported");

    let bind_addrs = (0..N)
        .map(|_| find_open_tcp_bind_port())
        .map(|port| {
            (
                Multiaddr::from_str(&format!("/ip4/{LOCAL_BIND_ADDR}/tcp/{port}"))
                    .expect("Should parse MultiAddr"),
                port,
            )
        })
        .collect::<Vec<_>>();

    let multi_addrs = bind_addrs
        .iter()
        .map(|(addr, _)| addr.clone())
        .collect::<Vec<_>>();

    let mut handles = vec![];

    for (node_index, (my_addr, my_port)) in bind_addrs.iter().enumerate() {
        let my_alias = NAME_IDS[node_index];

        let test_input = PerTestNodeInput {
            instance_id: node_index as _,
            bind_ip: IpAddr::from_str(LOCAL_BIND_ADDR).expect("Should be a valid IP"),
            bind_port: *my_port,
            bootnodes: multi_addrs
                .iter()
                .filter(|addr| *addr != my_addr)
                .cloned()
                .collect(),
            // Assumes that that tangle has initialized a dir with a keystore at ../../tangle/tmp/
            base_path: format!("../../tangle/tmp/{my_alias}"),
            verbose: 4,
            pretty: false,
            extra_input: additional_params.clone(),
            local_tangle_node: Url::parse(&opts.rpc_url).expect("Should parse URL"),
        };

        let handle = f(test_input).await;

        if node_index == 0 {
            // Replace the None signer and signer_evm values inside opts with alice's keys
            opts.signer = Some(handle.sr25519_id().clone());
            let k256_ecdsa_secret_key = handle.ecdsa_id().seed();
            opts.signer_evm = Some(
                PrivateKeySigner::from_slice(&k256_ecdsa_secret_key)
                    .expect("Should create a private key signer"),
            );
        }

        handles.push(handle);
    }

    let client = OnlineClient::<SubstrateConfig>::from_url(LOCAL_TANGLE_NODE)
        .await
        .expect("Failed to create primary localhost client");

    // Step 1: Create the blueprint using alice's identity
    let blueprint_id = cargo_tangle::deploy::deploy_to_tangle(opts)
        .await
        .expect("Failed to deploy Blueprint to Tangle");

    // Step 2: Have each identity register to a blueprint
    let registration_args = RegistrationArgs::new();

    for handle in handles.iter() {
        let keypair = handle.sr25519_id().clone();
        let key = runtime_types::sp_core::ecdsa::Public(handle.ecdsa_id().public().0);

        let preferences = Preferences {
            key,
            approval: ApprovalPrefrence::None,
            price_targets: PriceTargets {
                cpu: 0,
                mem: 0,
                storage_hdd: 0,
                storage_ssd: 0,
                storage_nvme: 0,
            },
        };

        super::register_blueprint(
            &client,
            &keypair,
            blueprint_id,
            preferences,
            registration_args.clone(),
        )
        .await
        .expect("Failed to register to blueprint");
    }

    // Now, start every blueprint manager. With the blueprint submitted and every operator registered
    // to the blueprint, we can now start the blueprint manager, expecting that the blueprint manager
    // will start the services associated with the blueprint as gadgets.
    for handle in handles.iter_mut() {
        handle.start().expect("Failed to start blueprint manager");
    }

    LocalhostTestExt { client, handles }
}

fn find_open_tcp_bind_port() -> u16 {
    let listener = std::net::TcpListener::bind(format!("{LOCAL_BIND_ADDR}:0"))
        .expect("Should bind to localhost");
    listener
        .local_addr()
        .expect("Should have a local address")
        .port()
}

/// - `N`: number of nodes
/// - `K`: Number of networks accessible per node (should be equal to the number of services in a given blueprint)
/// - `D`: Any data that you want to pass with NodeInput.
/// - `F`: A function that generates a service's execution via a series of shells. Each shell executes a subset of the service,
///        as each service may have a set of operations that are executed in parallel, sequentially, or concurrently.
pub async fn new_test_ext<
    const N: usize,
    const K: usize,
    D: Send + Clone + 'static,
    F: Fn(NodeInput<MockNetwork, InMemoryBackend, D>) -> Fut,
    Fut: SendFuture<'static, BlueprintManagerHandle>,
>(
    additional_params: D,
    f: F,
) -> LocalhostTestExt {
    let role_pairs = (0..N)
        .map(|i| id_to_ecdsa_pair(i as u8))
        .collect::<Vec<_>>();
    let roles_identities = role_pairs
        .iter()
        .map(|pair| pair.public())
        .collect::<Vec<_>>();

    let pairs = (0..N)
        .map(|i| id_to_sr25519_pair(i as u8))
        .collect::<Vec<_>>();

    let networks = (0..K)
        .map(|_| MockNetwork::setup(&roles_identities))
        .collect::<Vec<_>>();

    // Transpose networks
    let networks = (0..N)
        .map(|i| {
            networks
                .iter()
                .map(|network| network[i].clone())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    // Each client connects to ws://127.0.0.1:9944. This client is for the test environment
    let client = OnlineClient::<SubstrateConfig>::from_url(LOCAL_TANGLE_NODE)
        .await
        .expect("Failed to create primary localhost client");

    let mut handles = vec![];

    for (node_index, ((role_pair, pair), networks)) in
        role_pairs.into_iter().zip(pairs).zip(networks).enumerate()
    {
        let account_id: AccountId32 = pair.public().0.into();
        let mut localhost_clients = Vec::new();

        for _ in 0..K {
            // Each client connects to ws://127.0.0.1:9944
            let client = OnlineClient::<SubstrateConfig>::from_url(LOCAL_TANGLE_NODE)
                .await
                .expect("Failed to create localhost client");

            let client = TangleRuntimeClient::new(client, account_id.clone());
            localhost_clients.push(client);
        }

        let logger = Logger {
            target: "blueprint-test-utils",
            id: format!("Peer {node_index}"),
        };

        let keystore = ECDSAKeyStore::in_memory(role_pair);
        let prometheus_config = PrometheusConfig::Disabled;

        let input = NodeInput {
            clients: localhost_clients,
            networks,
            account_id: sr25519::Public::from_raw(account_id.0),
            logger,
            keystore,
            node_index,
            additional_params: additional_params.clone(),
            prometheus_config,
        };

        let handle = f(input).await;
        handles.push(handle);
    }

    LocalhostTestExt { handles, client }
}

pub struct LocalhostTestExt {
    client: OnlineClient<SubstrateConfig>,
    handles: Vec<BlueprintManagerHandle>,
}

impl LocalhostTestExt {
    /// An identity function (For future reverse-compatible changes)
    pub fn execute_with<
        T: FnOnce(&OnlineClient<SubstrateConfig>, &Vec<BlueprintManagerHandle>) -> R + Send + 'static,
        R: Send + 'static,
    >(
        &self,
        function: T,
    ) -> R {
        function(&self.client, &self.handles)
    }

    /// An identity function (For future reverse-compatible changes)
    pub async fn execute_with_async<
        'a,
        'b: 'a,
        T: FnOnce(&'a OnlineClient<SubstrateConfig>, &'a Vec<BlueprintManagerHandle>) -> R + Send + 'a,
        R: Future<Output = Out> + Send + 'a,
        Out: Send + 'b,
    >(
        &'a self,
        function: T,
    ) -> Out {
        function(&self.client, &self.handles).await
    }
}
