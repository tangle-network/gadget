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
use async_trait::async_trait;
use blueprint_manager::executor::BlueprintManagerHandle;
use cargo_tangle::deploy::{Opts, PrivateKeySigner};
use environment_utils::transaction_manager::tangle::SubxtPalletSubmitter;
use futures::stream::FuturesOrdered;
use futures::StreamExt;
use gadget_common::config::Network;
use gadget_common::locks::TokioMutexExt;
use gadget_common::prelude::{
    DebugLogger, ECDSAKeyStore, GadgetEnvironment, InMemoryBackend, NodeInput, PairSigner,
    PrometheusConfig, UnboundedReceiver, UnboundedSender,
};
use gadget_common::tangle_runtime::api;
use gadget_common::tangle_runtime::api::runtime_types::tangle_primitives::services::ApprovalPrefrence;
use gadget_common::tangle_runtime::api::services::calls::types::register::{
    Preferences, RegistrationArgs,
};
use gadget_common::tangle_subxt::subxt::ext::subxt_core::utils;
use gadget_common::tangle_subxt::subxt::{OnlineClient, SubstrateConfig};
use gadget_common::Error;
use gadget_core::job::protocol::{ProtocolMessageMetadata, WorkManagerInterface};
use gadget_core::job::SendFuture;
use libp2p::Multiaddr;
use sp_application_crypto::ecdsa;
use sp_core::{sr25519, Pair};
use std::collections::HashMap;
use std::future::Future;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tangle_environment::runtime::{TangleConfig, TangleRuntime};
use tangle_environment::TangleEnvironment;
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

pub struct MockNetwork<Env: GadgetEnvironment> {
    peers_tx: Arc<
        HashMap<
            ecdsa::Public,
            UnboundedSender<<Env::WorkManager as WorkManagerInterface>::ProtocolMessage>,
        >,
    >,
    peers_rx: PeersRx<<Env::WorkManager as WorkManagerInterface>::ProtocolMessage>,
    my_id: ecdsa::Public,
}

impl<Env: GadgetEnvironment> Clone for MockNetwork<Env> {
    fn clone(&self) -> Self {
        Self {
            peers_tx: self.peers_tx.clone(),
            peers_rx: self.peers_rx.clone(),
            my_id: self.my_id,
        }
    }
}

impl<Env: GadgetEnvironment> MockNetwork<Env> {
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

#[async_trait]
impl<Env: GadgetEnvironment> Network<Env> for MockNetwork<Env>
where
    Env::ProtocolMessage: Clone,
{
    async fn next_message(
        &self,
    ) -> Option<<Env::WorkManager as WorkManagerInterface>::ProtocolMessage> {
        self.peers_rx
            .get(&self.my_id)?
            .lock_timeout(Duration::from_millis(500))
            .await
            .recv()
            .await
    }

    async fn send_message(
        &self,
        message: <Env::WorkManager as WorkManagerInterface>::ProtocolMessage,
    ) -> Result<(), Error> {
        let _check_message_has_ids = message.sender_network_id().ok_or(Error::MissingNetworkId)?;
        if let Some(peer_id) = message.recipient_network_id() {
            let tx = self
                .peers_tx
                .get(&peer_id)
                .ok_or(Error::PeerNotFound { id: peer_id })?;
            tx.send(message).map_err(|err| Error::NetworkError {
                err: err.to_string(),
            })?;
        } else {
            // Broadcast to everyone except ourself
            for (peer_id, tx) in self.peers_tx.iter() {
                if peer_id != &self.my_id {
                    tx.send(message.clone())
                        .map_err(|err| Error::NetworkError {
                            err: err.to_string(),
                        })?;
                }
            }
        }
        Ok(())
    }
}

const LOCAL_BIND_ADDR: &str = "127.0.0.1";
const LOCAL_TANGLE_NODE: &str = "ws://127.0.0.1:9944";
pub const NAME_IDS: [&str; 5] = ["Alice", "Bob", "Charlie", "Dave", "Eve"];

/// N: number of nodes
/// K: Number of networks accessible per node (should be equal to the number of services in a given blueprint)
/// D: Any data that you want to pass to pass with NodeInput.
/// F: A function that generates a service's execution via a series of shells. Each shell executes a subset of the service,
/// as each service may have a set of operations that are executed in parallel, sequentially, or concurrently.
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
    assert!(N > 0, "At least one node is required");
    assert!(N <= NAME_IDS.len(), "Only up to 5 nodes are supported");

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

        let k256_ecdsa_secret_key = handle.ecdsa_id().0.secret_bytes();
        let priv_key = PrivateKeySigner::from_slice(&k256_ecdsa_secret_key)
            .expect("Should create a private key signer");

        let tg_addr = handle.sr25519_id().public_key().to_account_id();
        let evm_addr = handle.ecdsa_id().public_key().to_account_id();
        handle
            .logger()
            .info(format!("Signer TG address: {tg_addr}"));
        handle
            .logger()
            .info(format!("Signer EVM address: {evm_addr}"));
        handle
            .logger()
            .info(format!("Signer EVM(alloy) address: {}", priv_key.address()));

        if node_index == 0 {
            // Replace the None signer and signer_evm values inside opts with Alice's keys
            opts.signer_evm = Some(priv_key);
            opts.signer = Some(handle.sr25519_id().clone());
        }

        handles.push(handle);
    }

    // Step 1: Create the blueprint using alice's identity
    let blueprint_id = cargo_tangle::deploy::deploy_to_tangle(opts)
        .await
        .expect("Failed to deploy Blueprint to Tangle");

    // Step 2: Have each identity register to a blueprint
    let mut futures_ordered = FuturesOrdered::new();
    let registration_args = RegistrationArgs::new();
    // TODO: allow the function callee to specify the registration args

    for handle in handles {
        let client = OnlineClient::<SubstrateConfig>::from_url(LOCAL_TANGLE_NODE)
            .await
            .expect("Failed to create an account-based localhost client");
        let registration_args = registration_args.clone();

        let task = async move {
            let keypair = handle.sr25519_id().clone();
            let key = api::runtime_types::sp_core::ecdsa::Public(handle.ecdsa_id().public_key().0);

            let preferences = Preferences {
                key,
                approval: ApprovalPrefrence::None,
            };

            if let Err(err) = super::register_blueprint(
                &client,
                &keypair,
                blueprint_id,
                preferences,
                registration_args.clone(),
                handle.logger(),
            )
            .await
            {
                let err_str = format!("{err}");
                if err_str.contains("MultiAssetDelegation::AlreadyOperator") {
                    handle.logger().warn(format!(
                        "{} is already an operator",
                        keypair.public_key().to_account_id()
                    ));
                } else {
                    handle
                        .logger()
                        .error(format!("Failed to register as operator: {err}"));
                    panic!("Failed to register as operator: {err}");
                }
            }

            handle
        };

        futures_ordered.push_back(task);
    }

    let mut handles = futures_ordered
        .collect::<Vec<BlueprintManagerHandle>>()
        .await;

    let client = OnlineClient::<SubstrateConfig>::from_url(LOCAL_TANGLE_NODE)
        .await
        .expect("Failed to create primary localhost client");

    // Step 3: register a service
    let all_nodes = handles
        .iter()
        .map(|handle| handle.sr25519_id().public_key().to_account_id())
        .collect();

    // Use Alice's account to register the service
    handles[0].logger().info(format!(
        "Registering service for blueprint ID {blueprint_id} using Alice's keys ..."
    ));
    if let Err(err) =
        super::register_service(&client, handles[0].sr25519_id(), blueprint_id, all_nodes).await
    {
        handles[0]
            .logger()
            .error(format!("Failed to register service: {err}"));
        panic!("Failed to register service: {err}");
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

/// N: number of nodes
/// K: Number of networks accessible per node (should be equal to the number of services in a given blueprint)
/// D: Any data that you want to pass with NodeInput.
/// F: A function that generates a service's execution via a series of shells. Each shell executes a subset of the service,
/// as each service may have a set of operations that are executed in parallel, sequentially, or concurrently.
pub async fn new_test_ext<
    const N: usize,
    const K: usize,
    D: Send + Clone + 'static,
    F: Fn(NodeInput<TangleEnvironment, MockNetwork<TangleEnvironment>, InMemoryBackend, D>) -> Fut,
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

    /*
    let account_ids = pairs
        .iter()
        .map(|pair| pair.public().into())
        .collect::<Vec<AccountId>>();


    let balances = account_ids
        .iter()
        .map(|public| (public.clone(), 100u128))
        .collect::<Vec<_>>();*/

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
        let account_id: utils::AccountId32 = pair.public().0.into();
        let mut localhost_clients = Vec::new();

        for _ in 0..K {
            // Each client connects to ws://127.0.0.1:9944
            let client = OnlineClient::<SubstrateConfig>::from_url(LOCAL_TANGLE_NODE)
                .await
                .expect("Failed to create localhost client");

            let client = TangleRuntime::new(client, account_id.clone());
            localhost_clients.push(client);
        }

        let logger = DebugLogger {
            id: format!("Peer {node_index}"),
        };

        let pair_signer = PairSigner::<TangleConfig>::new(pair);

        let tx_manager = Arc::new(
            SubxtPalletSubmitter::new(pair_signer, logger.clone())
                .await
                .expect("Failed to create tx manager"),
        );

        let keystore = ECDSAKeyStore::in_memory(role_pair);
        let prometheus_config = PrometheusConfig::Disabled;

        let input = NodeInput {
            clients: localhost_clients,
            networks,
            account_id: sr25519::Public::from_raw(account_id.0),
            logger,
            tx_manager: tx_manager as _,
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
