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

use crate::sync::substrate_test_channel::MultiThreadedTestExternalities;
use async_trait::async_trait;
use environment_utils::transaction_manager::tangle::SubxtPalletSubmitter;
use frame_support::pallet_prelude::*;
use gadget_common::config::Network;
use gadget_common::locks::TokioMutexExt;
use gadget_common::prelude::{
    DebugLogger, ECDSAKeyStore, GadgetEnvironment, InMemoryBackend, NodeInput,
    PrometheusConfig, UnboundedReceiver, UnboundedSender,
};
use gadget_common::tangle_subxt::subxt::{OnlineClient, SubstrateConfig};
use gadget_common::Error;
use gadget_core::job_manager::{ProtocolMessageMetadata, SendFuture, WorkManagerInterface};
use pallet_services::EvmRunner;
use sp_application_crypto::ecdsa;
use sp_core::{sr25519, Pair};
use sp_keystore::testing::MemoryKeystore;
use sp_keystore::{KeystoreExt, KeystorePtr};
use sp_runtime::traits::Block as BlockT;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::time::Duration;
use tangle_primitives::AccountId;
use tangle_environment::TangleEnvironment;

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

/// N: number of nodes
/// K: Number of networks accessible per node
/// D: Any data that you want to pass to pass with NodeInput.
/// F: A function that generates a service's execution via a series of shells. Each shell executes a subset of the service,
/// as each service may have a set of operations that are executed in parallel, sequentially, or concurrently.
pub async fn new_test_ext<
    const N: usize,
    const K: usize,
    D: Send + Clone + 'static,
    F: Fn(
        NodeInput<TangleEnvironment, MockNetwork<TangleEnvironment>, InMemoryBackend, D>,
    ) -> Fut,
    Fut: SendFuture<'static, ()>,
>(
    additional_params: D,
    f: F,
) -> MultiThreadedTestExternalities {
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
    let account_ids = pairs
        .iter()
        .map(|pair| pair.public().into())
        .collect::<Vec<AccountId>>();

    let balances = account_ids
        .iter()
        .map(|public| (public.clone(), 100u128))
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

    let mut ext = sp_io::TestExternalities::new(t);
    ext.register_extension(KeystoreExt(Arc::new(MemoryKeystore::new()) as KeystorePtr));

    let ext = MultiThreadedTestExternalities::new(ext);

    for (node_index, ((role_pair, pair), networks)) in
        role_pairs.into_iter().zip(pairs).zip(networks).enumerate()
    {
        let mut localhost_clients = Vec::new();

        for _ in 0..K {
            // Each client connects to ws://127.0.0.1:9944
            let client = OnlineClient::<SubstrateConfig>::new().await.expect("Failed to create localhost client");
            localhost_clients.push(client);
        }

        let account_id: AccountId = pair.public().into();

        let logger = DebugLogger {
            id: format!("Peer {node_index}"),
        };

        let tx_manager = Arc::new(Arc::new(
            SubxtPalletSubmitter::new(account_id.clone(), logger.clone()).await.expect("Failed to create tx manager"),
        ));

        let keystore = ECDSAKeyStore::in_memory(role_pair);
        let prometheus_config = PrometheusConfig::Disabled;

        let input = NodeInput {
            clients: localhost_clients,
            networks,
            account_id: sr25519::Public(account_id.into()),
            logger,
            tx_manager: tx_manager as _,
            keystore,
            node_index,
            additional_params: additional_params.clone(),
            prometheus_config,
        };

        let task = f(input);
        gadget_io::tokio::task::spawn(task);
    }

    ext
}

pub fn mock_pub_key(id: u8) -> AccountId {
    sr25519::Public::from_raw([id; 32]).into()
}

pub struct LocalhostTestExt {
    client: OnlineClient<SubstrateConfig>
}

impl LocalhostTestExt {
    /// An identity function (For future reverse-compatible changes)
    pub fn execute_with<T: FnOnce() -> R + Send + 'static, R: Send + 'static>(
        &self,
        function: T,
    ) -> R {
        function()
    }

    /// An identity function (For future reverse-compatible changes)
    pub async fn execute_with_async<T: FnOnce() -> R + Send + 'static, R: Send + 'static>(
        &self,
        function: T,
    ) -> R {
        function()
    }
}

impl Deref for LocalhostTestExt {
    type Target = OnlineClient<SubstrateConfig>;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl DerefMut for LocalhostTestExt {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.client
    }
}

impl From<OnlineClient<SubstrateConfig>> for LocalhostTestExt {
    fn from(client: OnlineClient<SubstrateConfig>) -> Self {
        Self { client }
    }
}