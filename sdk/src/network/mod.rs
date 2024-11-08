use crate::error::Error;
use async_trait::async_trait;
use core::fmt::Display;
use dashmap::DashMap;
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use sp_core::{ecdsa, sha2_256};
use std::ops::Deref;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::Mutex;

use self::channels::UserID;

pub mod channels;
pub mod gossip;
pub mod handlers;
#[cfg(target_family = "wasm")]
pub mod matchbox;
pub mod messaging;
pub mod setup;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub struct IdentifierInfo {
    pub block_id: Option<u64>,
    pub session_id: Option<u64>,
    pub retry_id: Option<u64>,
    pub task_id: Option<u64>,
}

impl Display for IdentifierInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let block_id = self
            .block_id
            .map(|id| format!("block_id: {}", id))
            .unwrap_or_default();
        let session_id = self
            .session_id
            .map(|id| format!("session_id: {}", id))
            .unwrap_or_default();
        let retry_id = self
            .retry_id
            .map(|id| format!("retry_id: {}", id))
            .unwrap_or_default();
        let task_id = self
            .task_id
            .map(|id| format!("task_id: {}", id))
            .unwrap_or_default();
        write!(f, "{} {} {} {}", block_id, session_id, retry_id, task_id)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct ParticipantInfo {
    pub user_id: u16,
    pub ecdsa_key: Option<sp_core::ecdsa::Public>,
}

impl Display for ParticipantInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ecdsa_key = self
            .ecdsa_key
            .map(|key| format!("ecdsa_key: {}", key))
            .unwrap_or_default();
        write!(f, "user_id: {}, {}", self.user_id, ecdsa_key)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProtocolMessage {
    pub identifier_info: IdentifierInfo,
    pub sender: ParticipantInfo,
    pub recipient: Option<ParticipantInfo>,
    pub payload: Vec<u8>,
}

impl Display for ProtocolMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "identifier_info: {}, sender: {}, recipient: {:?}, payload: {:?}",
            self.identifier_info, self.sender, self.recipient, self.payload
        )
    }
}

#[async_trait]
pub trait Network: Send + Sync + 'static {
    async fn next_message(&self) -> Option<ProtocolMessage>;
    async fn send_message(&self, message: ProtocolMessage) -> Result<(), Error>;

    fn build_protocol_message<Payload: Serialize>(
        identifier_info: IdentifierInfo,
        from: UserID,
        to: Option<UserID>,
        payload: &Payload,
        from_account_id: Option<ecdsa::Public>,
        to_network_id: Option<ecdsa::Public>,
    ) -> ProtocolMessage {
        let sender_participant_info = ParticipantInfo {
            user_id: from,
            ecdsa_key: from_account_id,
        };
        let receiver_participant_info = to.map(|to| ParticipantInfo {
            user_id: to,
            ecdsa_key: to_network_id,
        });
        ProtocolMessage {
            identifier_info,
            sender: sender_participant_info,
            recipient: receiver_participant_info,
            payload: serialize(payload).expect("Failed to serialize message"),
        }
    }
}

pub struct NetworkMultiplexer {
    to_receiving_streams: ActiveStreams,
    unclaimed_receiving_streams: Arc<DashMap<StreamKey, MultiplexedReceiver>>,
    tx_to_networking_layer: MultiplexedSender,
}

type ActiveStreams = Arc<DashMap<StreamKey, tokio::sync::mpsc::UnboundedSender<ProtocolMessage>>>;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Default, Serialize, Deserialize)]
pub struct StreamKey {
    pub task_hash: [u8; 32],
    pub round_id: i32,
}

impl From<IdentifierInfo> for StreamKey {
    fn from(identifier_info: IdentifierInfo) -> Self {
        let str_repr = identifier_info.to_string();
        let task_hash = sha2_256(str_repr.as_bytes());
        Self {
            task_hash,
            round_id: -1,
        }
    }
}

pub struct MultiplexedReceiver {
    inner: tokio::sync::mpsc::UnboundedReceiver<ProtocolMessage>,
    stream_id: StreamKey,
    // For post-drop removal purposes
    active_streams: ActiveStreams,
}

#[derive(Clone)]
pub struct MultiplexedSender {
    inner: tokio::sync::mpsc::UnboundedSender<(StreamKey, ProtocolMessage)>,
    pub(crate) stream_id: StreamKey,
}

impl MultiplexedSender {
    pub fn send(&self, message: ProtocolMessage) -> Result<(), Error> {
        self.inner
            .send((self.stream_id, message))
            .map_err(|err| Error::Other(err.to_string()))
    }
}

impl Stream for MultiplexedReceiver {
    type Item = ProtocolMessage;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().inner).poll_recv(cx)
    }
}

impl Deref for MultiplexedReceiver {
    type Target = tokio::sync::mpsc::UnboundedReceiver<ProtocolMessage>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Drop for MultiplexedReceiver {
    fn drop(&mut self) {
        let _ = self.active_streams.remove(&self.stream_id);
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct MultiplexedMessage {
    payload: Vec<u8>,
    stream_id: StreamKey,
}

impl NetworkMultiplexer {
    pub fn new<N: Network>(network: N) -> Self {
        let (tx_to_networking_layer, mut rx_from_substreams) =
            tokio::sync::mpsc::unbounded_channel();
        let this = NetworkMultiplexer {
            to_receiving_streams: Arc::new(DashMap::new()),
            unclaimed_receiving_streams: Arc::new(DashMap::new()),
            tx_to_networking_layer: MultiplexedSender {
                inner: tx_to_networking_layer,
                stream_id: Default::default(), // Start with an arbitrary stream ID, this won't get used
            },
        };

        let active_streams = this.to_receiving_streams.clone();
        let unclaimed_streams = this.unclaimed_receiving_streams.clone();
        let tx_to_networking_layer = this.tx_to_networking_layer.clone();
        drop(tokio::spawn(async move {
            let network_clone = &network;

            let task1 = async move {
                while let Some((stream_id, proto_message)) = rx_from_substreams.recv().await {
                    let multiplexed_message = MultiplexedMessage {
                        payload: proto_message.payload,
                        stream_id,
                    };
                    let message = ProtocolMessage {
                        identifier_info: proto_message.identifier_info,
                        sender: proto_message.sender,
                        recipient: proto_message.recipient,
                        payload: bincode2::serialize(&multiplexed_message)
                            .expect("Failed to serialize message"),
                    };

                    if let Err(err) = network_clone.send_message(message).await {
                        crate::error!(%err, "Failed to send message to network");
                        break;
                    }
                }
            };

            let task2 = async move {
                while let Some(mut msg) = network_clone.next_message().await {
                    if let Ok(multiplexed_message) =
                        bincode2::deserialize::<MultiplexedMessage>(&msg.payload)
                    {
                        let stream_id = multiplexed_message.stream_id;
                        msg.payload = multiplexed_message.payload;
                        // Two possibilities: the entry already exists, or, it doesn't and we need to enqueue
                        if let Some(active_receiver) = active_streams.get(&stream_id) {
                            if let Err(err) = active_receiver.send(msg) {
                                crate::error!(%err, "Failed to send message to receiver");
                                // Delete entry since the receiver is dead
                                let _ = active_streams.remove(&stream_id);
                            }
                        } else {
                            // Second possibility: the entry does not exist, and another substream is received for this task.
                            // In this case, reserve an entry locally and store the message in the unclaimed streams. Later,
                            // when the user attempts to open the substream with the same ID, the message will be sent to the user.
                            let (tx, rx) = Self::create_multiplexed_stream_inner(
                                tx_to_networking_layer.clone(),
                                &active_streams,
                                stream_id,
                            );
                            let _ = tx.send(msg);
                            //let _ = active_streams.insert(stream_id, tx); TX already passed into active_streams above
                            let _ = unclaimed_streams.insert(stream_id, rx);
                        }
                    }
                }
            };

            tokio::select! {
                _ = task1 => {
                    crate::error!("Task 1 exited");
                },
                _ = task2 => {
                    crate::error!("Task 2 exited");
                }
            }
        }));

        this
    }

    pub fn multiplex(&self, id: impl Into<StreamKey>) -> SubNetwork {
        let id = id.into();
        let mut tx_to_networking_layer = self.tx_to_networking_layer.clone();
        if let Some(unclaimed) = self.unclaimed_receiving_streams.remove(&id) {
            tx_to_networking_layer.stream_id = id;
            return SubNetwork {
                tx: tx_to_networking_layer,
                rx: unclaimed.1.into(),
            };
        }

        let (tx, rx) = Self::create_multiplexed_stream_inner(
            tx_to_networking_layer,
            &self.to_receiving_streams,
            id,
        );

        SubNetwork { tx, rx: rx.into() }
    }

    fn create_multiplexed_stream_inner(
        mut tx_to_networking_layer: MultiplexedSender,
        active_streams: &ActiveStreams,
        stream_id: StreamKey,
    ) -> (MultiplexedSender, MultiplexedReceiver) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        if active_streams.insert(stream_id, tx).is_some() {
            crate::warn!(
                "Stream ID {stream_id:?} already exists! Existing stream will be replaced"
            );
        }
        tx_to_networking_layer.stream_id = stream_id;

        (
            tx_to_networking_layer,
            MultiplexedReceiver {
                inner: rx,
                stream_id,
                active_streams: active_streams.clone(),
            },
        )
    }
}

impl<N: Network> From<N> for NetworkMultiplexer {
    fn from(network: N) -> Self {
        Self::new(network)
    }
}

pub struct SubNetwork {
    tx: MultiplexedSender,
    rx: Mutex<MultiplexedReceiver>,
}

impl SubNetwork {
    pub fn send(&self, message: ProtocolMessage) -> Result<(), Error> {
        self.tx.send(message)
    }

    pub async fn recv(&self) -> Option<ProtocolMessage> {
        self.rx.lock().await.next().await
    }
}

#[async_trait]
impl Network for SubNetwork {
    async fn next_message(&self) -> Option<ProtocolMessage> {
        self.recv().await
    }

    async fn send_message(&self, message: ProtocolMessage) -> Result<(), Error> {
        self.send(message)
    }
}

impl Stream for SubNetwork {
    type Item = ProtocolMessage;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(self.rx.get_mut()).poll_next(cx)
    }
}

pub fn deserialize<'a, T>(data: &'a [u8]) -> Result<T, serde_json::Error>
where
    T: Deserialize<'a>,
{
    serde_json::from_slice::<T>(data)
}

pub fn serialize(object: &impl Serialize) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(object)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{stream, StreamExt};
    use gossip::GossipHandle;
    use serde::{Deserialize, Serialize};
    use sp_core::Pair;
    use std::collections::BTreeMap;

    const TOPIC: &str = "/gadget/test/1.0.0";

    #[derive(Debug, Serialize, Deserialize, Clone)]
    enum Msg {
        Round1(Round1Msg),
        Round2(Round2Msg),
        Round3(Round3Msg),
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    struct Round1Msg {
        pub power: u16,
        pub hitpoints: u16,
        pub armor: u16,
        pub name: String,
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    struct Round2Msg {
        pub x: u16,
        pub y: u16,
        pub z: u16,
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    struct Round3Msg {
        rotation: u16,
        velocity: (u16, u16, u16),
    }

    // NOTE: if you lower the number of nodes to 2, this test passes without issues.
    const NODE_COUNT: u16 = 10;

    pub fn setup_log() {
        use tracing_subscriber::util::SubscriberInitExt;
        let env_filter = tracing_subscriber::EnvFilter::from_default_env()
            .add_directive("tokio=off".parse().unwrap())
            .add_directive("hyper=off".parse().unwrap())
            .add_directive("gadget=debug".parse().unwrap());

        let _ = tracing_subscriber::fmt::SubscriberBuilder::default()
            .compact()
            .without_time()
            .with_span_events(tracing_subscriber::fmt::format::FmtSpan::NONE)
            .with_target(false)
            .with_env_filter(env_filter)
            .with_test_writer()
            .finish()
            .try_init();
    }

    async fn wait_for_nodes_connected(nodes: &[GossipHandle]) {
        let node_count = nodes.len();

        // wait for the nodes to connect to each other
        let max_retries = 30 * node_count;
        let mut retry = 0;
        loop {
            crate::debug!(%node_count, %max_retries, %retry, "Checking if all nodes are connected to each other");
            let connected = nodes
                .iter()
                .map(|node| node.connected_peers())
                .collect::<Vec<_>>();

            let all_connected = connected
                .iter()
                .enumerate()
                .inspect(|(node, peers)| crate::debug!("Node {node} has {peers} connected peers"))
                .all(|(_, &peers)| peers == node_count - 1);
            if all_connected {
                crate::debug!("All nodes are connected to each other");
                return;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
            retry += 1;
            if retry > max_retries {
                panic!("Failed to connect all nodes to each other");
            }
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_p2p() {
        setup_log();
        let nodes = stream::iter(0..NODE_COUNT)
            .map(|_| node())
            .collect::<Vec<_>>()
            .await;

        wait_for_nodes_connected(&nodes).await;

        let mut tasks = Vec::new();
        for (i, node) in nodes.into_iter().enumerate() {
            let task = tokio::spawn(run_protocol(node, i as u16));
            tasks.push(task);
        }
        // Wait for all tasks to finish
        let results = futures::future::try_join_all(tasks)
            .await
            .expect("Failed to run protocol");
        // Assert that all are okay.
        assert!(
            results.iter().all(|r| r.is_ok()),
            "Some nodes failed to run protocol"
        );
    }

    async fn run_protocol<N: Network>(node: N, i: u16) -> Result<(), crate::Error> {
        let task_hash = [0u8; 32];
        // Safety note: We should be passed a NetworkMultiplexer, and all uses of the N: Network
        // used throughout the program must also use the multiplexer to prevent mixed messages.
        let multiplexer = NetworkMultiplexer::new(node);

        let mut round1_network = multiplexer.multiplex(StreamKey {
            task_hash, // To differentiate between different instances of a running program (i.e., a task)
            round_id: 0, // To differentiate between different subsets of a running task
        });

        let mut round2_network = multiplexer.multiplex(StreamKey {
            task_hash, // To differentiate between different instances of a running program (i.e., a task)
            round_id: 1, // To differentiate between different subsets of a running task
        });

        let mut round3_network = multiplexer.multiplex(StreamKey {
            task_hash, // To differentiate between different instances of a running program (i.e., a task)
            round_id: 2, // To differentiate between different subsets of a running task
        });

        //let (round1_tx, round1_rx) = node.
        // Round 1 (broadcast)
        let msg = {
            let round = Round1Msg {
                power: i * 100,
                hitpoints: (i + 1) * 50,
                armor: i + 2,
                name: format!("Player {}", i),
            };

            GossipHandle::build_protocol_message(
                IdentifierInfo {
                    block_id: None,
                    session_id: None,
                    retry_id: None,
                    task_id: None,
                },
                i,
                None,
                &Msg::Round1(round),
                None,
                None,
            )
        };

        crate::debug!("Broadcast Message");
        round1_network
            .send(msg)
            .map_err(|_| crate::Error::Other("Failed to send message".into()))?;

        // Wait for all other nodes to send their messages
        let mut msgs = BTreeMap::new();
        while let Some(msg) = round1_network.next().await {
            let m = deserialize::<Msg>(&msg.payload).unwrap();
            crate::debug!(from = %msg.sender.user_id, ?m, "Received message");
            // Expecting Round1 message
            assert!(
                matches!(m, Msg::Round1(_)),
                "Expected Round1 message but got {:?} from node {}",
                m,
                msg.sender.user_id,
            );
            let old = msgs.insert(msg.sender.user_id, m);
            assert!(
                old.is_none(),
                "Duplicate message from node {}",
                msg.sender.user_id
            );
            // Break if all messages are received
            if msgs.len() == usize::from(NODE_COUNT) - 1 {
                break;
            }
        }
        crate::debug!("Done r1 w/ {i}");

        // Round 2 (P2P)
        let msg = Round2Msg {
            x: i * 10,
            y: (i + 1) * 20,
            z: i + 2,
        };
        let msgs = (0..NODE_COUNT)
            .filter(|&j| j != i)
            .map(|j| {
                GossipHandle::build_protocol_message(
                    IdentifierInfo {
                        block_id: None,
                        session_id: None,
                        retry_id: None,
                        task_id: None,
                    },
                    i,
                    Some(j),
                    &Msg::Round2(msg.clone()),
                    None,
                    None,
                )
            })
            .collect::<Vec<_>>();
        for msg in msgs {
            let to = msg.recipient.map(|r| r.user_id).expect(
                "Recipient should be present for P2P message. This is a bug in the test code",
            );
            crate::debug!(%to, "Send P2P Message");
            round2_network.send(msg)?;
        }

        // Wait for all other nodes to send their messages
        let mut msgs = BTreeMap::new();
        while let Some(msg) = round2_network.next().await {
            let m = deserialize::<Msg>(&msg.payload).unwrap();
            crate::debug!(from = %msg.sender.user_id, ?m, "Received message");
            // Expecting Round2 message
            assert!(
                matches!(m, Msg::Round2(_)),
                "Expected Round2 message but got {:?} from node {}",
                m,
                msg.sender.user_id,
            );
            let old = msgs.insert(msg.sender.user_id, m);
            assert!(
                old.is_none(),
                "Duplicate message from node {}",
                msg.sender.user_id
            );
            // Break if all messages are received
            if msgs.len() == usize::from(NODE_COUNT) - 1 {
                break;
            }
        }
        crate::debug!("Done r2 w/ {i}");

        // Round 3 (broadcast)

        let msg = {
            let round = Round3Msg {
                rotation: i * 30,
                velocity: (i + 1, i + 2, i + 3),
            };
            GossipHandle::build_protocol_message(
                IdentifierInfo {
                    block_id: None,
                    session_id: None,
                    retry_id: None,
                    task_id: None,
                },
                i,
                None,
                &Msg::Round3(round),
                None,
                None,
            )
        };

        crate::debug!("Broadcast Message");
        round3_network.send(msg)?;

        // Wait for all other nodes to send their messages
        let mut msgs = BTreeMap::new();
        while let Some(msg) = round3_network.next().await {
            let m = deserialize::<Msg>(&msg.payload).unwrap();
            crate::debug!(from = %msg.sender.user_id, ?m, "Received message");
            // Expecting Round3 message
            assert!(
                matches!(m, Msg::Round3(_)),
                "Expected Round3 message but got {:?} from node {}",
                m,
                msg.sender.user_id,
            );
            let old = msgs.insert(msg.sender.user_id, m);
            assert!(
                old.is_none(),
                "Duplicate message from node {}",
                msg.sender.user_id
            );
            // Break if all messages are received
            if msgs.len() == usize::from(NODE_COUNT) - 1 {
                break;
            }
        }
        crate::debug!("Done r3 w/ {i}");

        crate::info!(node = i, "Protocol completed");

        Ok(())
    }

    fn node() -> gossip::GossipHandle {
        let identity = libp2p::identity::Keypair::generate_ed25519();
        let ecdsa_key = sp_core::ecdsa::Pair::generate().0;
        let bind_port = 0;
        setup::start_p2p_network(setup::NetworkConfig::new_service_network(
            identity,
            ecdsa_key,
            Default::default(),
            bind_port,
            TOPIC,
        ))
        .unwrap()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_nested_multiplexer() {
        setup_log();
        crate::info!("Starting test_nested_multiplexer");
        let network0 = node();
        let network1 = node();

        let mut networks = vec![network0, network1];

        wait_for_nodes_connected(&networks).await;

        let (network0, network1) = (networks.remove(0), networks.remove(0));

        async fn nested_multiplex<N: Network>(
            cur_depth: usize,
            max_depth: usize,
            network0: N,
            network1: N,
        ) {
            crate::info!("At nested depth = {cur_depth}/{max_depth}");

            if cur_depth == max_depth {
                return;
            }

            let multiplexer0 = NetworkMultiplexer::new(network0);
            let multiplexer1 = NetworkMultiplexer::new(network1);

            let stream_key = StreamKey {
                task_hash: sha2_256(&[(cur_depth % 255) as u8]),
                round_id: 0,
            };

            let subnetwork0 = multiplexer0.multiplex(stream_key);
            let subnetwork1 = multiplexer1.multiplex(stream_key);

            // Send a message in the subnetwork0 to subnetwork1 and vice versa, assert values of message
            let payload = vec![1, 2, 3];
            let msg = GossipHandle::build_protocol_message(
                IdentifierInfo::default(),
                0,
                Some(1),
                &payload,
                None,
                None,
            );

            subnetwork0.send(msg.clone()).unwrap();

            let received_msg = subnetwork1.recv().await.unwrap();
            assert_eq!(received_msg.payload, msg.payload);

            let msg = GossipHandle::build_protocol_message(
                IdentifierInfo::default(),
                1,
                Some(0),
                &payload,
                None,
                None,
            );

            subnetwork1.send(msg.clone()).unwrap();

            let received_msg = subnetwork0.recv().await.unwrap();
            assert_eq!(received_msg.payload, msg.payload);

            Box::pin(nested_multiplex(
                cur_depth + 1,
                max_depth,
                subnetwork0,
                subnetwork1,
            ))
            .await
        }

        nested_multiplex(0, 10, network0, network1).await;
    }
}
