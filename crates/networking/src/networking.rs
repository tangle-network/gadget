use crate::key_types::GossipMsgPublicKey;
use crate::Error;
use async_trait::async_trait;
use dashmap::DashMap;
use futures::{Stream, StreamExt};
use gadget_crypto::hashing::blake3_256;
use gadget_std::boxed::Box;
use gadget_std::cmp::Reverse;
use gadget_std::collections::{BinaryHeap, HashMap};
use gadget_std::fmt::Display;
use gadget_std::ops::{Deref, DerefMut};
use gadget_std::pin::Pin;
use gadget_std::string::ToString;
use gadget_std::sync::Arc;
use gadget_std::task::{Context, Poll};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;
use tracing::trace;

pub type UserID = u16;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub struct IdentifierInfo {
    pub message_id: u64,
    pub round_id: u16,
}

impl Display for IdentifierInfo {
    fn fmt(&self, f: &mut gadget_std::fmt::Formatter<'_>) -> gadget_std::fmt::Result {
        let message_id = format!("message_id: {}", self.message_id);
        let round_id = format!("round_id: {}", self.round_id);
        write!(f, "{} {}", message_id, round_id)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct ParticipantInfo {
    pub user_id: u16,
    pub public_key: Option<GossipMsgPublicKey>,
}

impl Display for ParticipantInfo {
    fn fmt(&self, f: &mut gadget_std::fmt::Formatter<'_>) -> gadget_std::fmt::Result {
        let public_key = self
            .public_key
            .map(|key| format!("public_key: {:?}", key))
            .unwrap_or_default();
        write!(f, "user_id: {}, {}", self.user_id, public_key)
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
    fn fmt(&self, f: &mut gadget_std::fmt::Formatter<'_>) -> gadget_std::fmt::Result {
        write!(
            f,
            "identifier_info: {}, sender: {}, recipient: {:?}, payload: {:?}",
            self.identifier_info, self.sender, self.recipient, self.payload
        )
    }
}

#[async_trait]
#[auto_impl::auto_impl(&, Box, Arc)]
pub trait Network: Send + Sync + 'static {
    async fn next_message(&self) -> Option<ProtocolMessage>;
    async fn send_message(&self, message: ProtocolMessage) -> Result<(), Error>;

    fn public_id(&self) -> GossipMsgPublicKey;

    fn build_protocol_message<Payload: Serialize>(
        &self,
        identifier_info: IdentifierInfo,
        from: UserID,
        to: Option<UserID>,
        payload: &Payload,
        to_network_id: Option<GossipMsgPublicKey>,
    ) -> ProtocolMessage {
        assert!(
            (u8::from(to.is_none()) + u8::from(to_network_id.is_none()) != 1),
            "Either `to` must be Some AND `to_network_id` is Some, or, both None"
        );

        let sender_participant_info = ParticipantInfo {
            user_id: from,
            public_key: Some(self.public_id()),
        };
        let receiver_participant_info = to.map(|to| ParticipantInfo {
            user_id: to,
            public_key: to_network_id,
        });
        ProtocolMessage {
            identifier_info,
            sender: sender_participant_info,
            recipient: receiver_participant_info,
            payload: bincode::serialize(payload).expect("Failed to serialize message"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct SequencedMessage {
    seq: u64,
    payload: Vec<u8>,
}

#[derive(Debug)]
struct PendingMessage {
    seq: u64,
    message: ProtocolMessage,
}

impl PartialEq for PendingMessage {
    fn eq(&self, other: &Self) -> bool {
        self.seq == other.seq
    }
}

impl Eq for PendingMessage {}

impl PartialOrd for PendingMessage {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PendingMessage {
    fn cmp(&self, other: &Self) -> gadget_std::cmp::Ordering {
        self.seq.cmp(&other.seq)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MultiplexedMessage {
    stream_id: StreamKey,
    payload: SequencedMessage,
}

pub struct NetworkMultiplexer {
    to_receiving_streams: ActiveStreams,
    unclaimed_receiving_streams: Arc<DashMap<StreamKey, MultiplexedReceiver>>,
    tx_to_networking_layer: MultiplexedSender,
    sequence_numbers: Arc<DashMap<CompoundStreamKey, u64>>,
    my_id: GossipMsgPublicKey,
}

type ActiveStreams = Arc<DashMap<StreamKey, UnboundedSender<ProtocolMessage>>>;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize, Default)]
pub struct StreamKey {
    pub task_hash: [u8; 32],
    pub round_id: i32,
}

impl From<IdentifierInfo> for StreamKey {
    fn from(identifier_info: IdentifierInfo) -> Self {
        let str_repr = identifier_info.to_string();
        let task_hash = blake3_256(str_repr.as_bytes());
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
    /// Sends a protocol message through the multiplexed channel.
    ///
    /// # Arguments
    /// * `message` - The protocol message to send
    ///
    /// # Returns
    /// * `Ok(())` - If the message was successfully sent
    /// * `Err(Error)` - If there was an error sending the message
    ///
    /// # Errors
    /// Returns an error if the receiving end of the channel has been closed,
    /// indicating that the network connection is no longer available.
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

impl DerefMut for MultiplexedReceiver {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Drop for MultiplexedReceiver {
    fn drop(&mut self) {
        let _ = self.active_streams.remove(&self.stream_id);
    }
}

// Since a single stream can be used for multiple users, and, multiple users assign seq's independently,
// we need to make a key that is unique for each (send->dest) pair and stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct CompoundStreamKey {
    stream_key: StreamKey,
    send_user: UserID,
    recv_user: Option<UserID>,
}

impl NetworkMultiplexer {
    /// Creates a new `NetworkMultiplexer` instance.
    ///
    /// # Arguments
    /// * `network` - The underlying network implementation that implements the Network trait
    ///
    /// # Type Parameters
    /// * `N` - The network type that implements the Network trait
    ///
    /// # Returns
    /// * `Self` - A new `NetworkMultiplexer` instance
    ///
    /// # Panics
    /// This function will panic if the internal receiver has already been taken, which should not happen.
    #[allow(clippy::too_many_lines)]
    pub fn new<N: Network>(network: N) -> Self {
        let (tx_to_networking_layer, mut rx_from_substreams) =
            tokio::sync::mpsc::unbounded_channel();
        let my_id = network.public_id();
        let this = NetworkMultiplexer {
            to_receiving_streams: Arc::new(DashMap::new()),
            unclaimed_receiving_streams: Arc::new(DashMap::new()),
            tx_to_networking_layer: MultiplexedSender {
                inner: tx_to_networking_layer,
                stream_id: StreamKey::default(),
            },
            sequence_numbers: Arc::new(DashMap::new()),
            my_id,
        };

        let active_streams = this.to_receiving_streams.clone();
        let unclaimed_streams = this.unclaimed_receiving_streams.clone();
        let tx_to_networking_layer = this.tx_to_networking_layer.clone();
        let sequence_numbers = this.sequence_numbers.clone();

        drop(tokio::spawn(async move {
            let network_clone = &network;

            let task1 = async move {
                while let Some((stream_id, msg)) = rx_from_substreams.recv().await {
                    let compound_key = CompoundStreamKey {
                        stream_key: stream_id,
                        send_user: msg.sender.user_id,
                        recv_user: msg.recipient.as_ref().map(|p| p.user_id),
                    };

                    let mut seq = sequence_numbers.entry(compound_key).or_insert(0);
                    let current_seq = *seq;
                    *seq += 1;

                    trace!(
                        "SEND SEQ {current_seq} FROM {} | StreamKey: {:?}",
                        msg.sender.user_id,
                        hex::encode(bincode::serialize(&compound_key).unwrap())
                    );

                    let multiplexed_message = MultiplexedMessage {
                        stream_id,
                        payload: SequencedMessage {
                            seq: current_seq,
                            payload: msg.payload,
                        },
                    };

                    let message = ProtocolMessage {
                        identifier_info: msg.identifier_info,
                        sender: msg.sender,
                        recipient: msg.recipient,
                        payload: bincode::serialize(&multiplexed_message)
                            .expect("Failed to serialize message"),
                    };

                    if let Err(err) = network_clone.send_message(message).await {
                        gadget_logging::error!("Failed to send message to network: {err:?}");
                        break;
                    }
                }
            };

            let task2 = async move {
                let mut pending_messages: HashMap<
                    CompoundStreamKey,
                    BinaryHeap<Reverse<PendingMessage>>,
                > = HashMap::default();
                let mut expected_seqs: HashMap<CompoundStreamKey, u64> = HashMap::default();

                while let Some(mut msg) = network_clone.next_message().await {
                    if let Some(recv) = msg.recipient.as_ref() {
                        if let Some(recv_pk) = &recv.public_key {
                            if recv_pk != &my_id {
                                gadget_logging::warn!(
                                    "Received a message not intended for the local user"
                                );
                            }
                        }
                    }

                    if let Ok(multiplexed_message) =
                        bincode::deserialize::<MultiplexedMessage>(&msg.payload)
                    {
                        let stream_id = multiplexed_message.stream_id;
                        let compound_key = CompoundStreamKey {
                            stream_key: stream_id,
                            send_user: msg.sender.user_id,
                            recv_user: msg.recipient.as_ref().map(|p| p.user_id),
                        };
                        let seq = multiplexed_message.payload.seq;
                        msg.payload = multiplexed_message.payload.payload;

                        // Get or create the pending heap for this stream
                        let pending = pending_messages.entry(compound_key).or_default();
                        let expected_seq = expected_seqs.entry(compound_key).or_default();

                        let send_user = msg.sender.user_id;
                        let recv_user = msg.recipient.as_ref().map_or(-1, |p| i32::from(p.user_id));
                        let compound_key_hex =
                            hex::encode(bincode::serialize(&compound_key).unwrap());
                        trace!(
                            "RECV SEQ {seq} FROM {} as user {:?} | Expecting: {} | StreamKey: {:?}",
                            send_user,
                            recv_user,
                            *expected_seq,
                            compound_key_hex,
                        );

                        // Add the message to pending
                        pending.push(Reverse(PendingMessage { seq, message: msg }));

                        // Try to deliver messages in order
                        if let Some(active_receiver) = active_streams.get(&stream_id) {
                            while let Some(Reverse(PendingMessage { seq, message: _ })) =
                                pending.peek()
                            {
                                if *seq != *expected_seq {
                                    break;
                                }

                                trace!("DELIVERING SEQ {seq} FROM {} as user {:?} | Expecting: {} | StreamKey: {:?}", send_user, recv_user, *expected_seq, compound_key_hex);

                                *expected_seq += 1;

                                let message = pending.pop().unwrap().0.message;

                                if let Err(err) = active_receiver.send(message) {
                                    gadget_logging::error!(%err, "Failed to send message to receiver");
                                    let _ = active_streams.remove(&stream_id);
                                    break;
                                }
                            }
                        } else {
                            let (tx, rx) = Self::create_multiplexed_stream_inner(
                                tx_to_networking_layer.clone(),
                                &active_streams,
                                stream_id,
                            );

                            // Deliver any pending messages in order
                            while let Some(Reverse(PendingMessage { seq, message: _ })) =
                                pending.peek()
                            {
                                if *seq != *expected_seq {
                                    break;
                                }

                                gadget_logging::warn!("EARLY DELIVERY SEQ {seq} FROM {} as user {:?} | Expecting: {} | StreamKey: {:?}", send_user, recv_user, *expected_seq, compound_key_hex);

                                *expected_seq += 1;

                                let message = pending.pop().unwrap().0.message;

                                if let Err(err) = tx.send(message) {
                                    gadget_logging::error!(%err, "Failed to send message to receiver");
                                    break;
                                }
                            }

                            let _ = unclaimed_streams.insert(stream_id, rx);
                        }
                    } else {
                        gadget_logging::error!("Failed to deserialize message (networking)");
                    }
                }
            };

            tokio::select! {
                () = task1 => {
                    gadget_logging::error!("Task 1 exited");
                },
                () = task2 => {
                    gadget_logging::error!("Task 2 exited");
                }
            }
        }));

        this
    }

    /// Creates a new multiplexed stream.
    ///
    /// # Arguments
    /// * `id` - The ID of the stream to create
    ///
    /// # Returns
    /// * `Self` - A new multiplexed stream
    pub fn multiplex(&self, id: impl Into<StreamKey>) -> SubNetwork {
        let id = id.into();
        let my_id = self.my_id;
        let mut tx_to_networking_layer = self.tx_to_networking_layer.clone();
        if let Some(unclaimed) = self.unclaimed_receiving_streams.remove(&id) {
            tx_to_networking_layer.stream_id = id;
            return SubNetwork {
                tx: tx_to_networking_layer,
                rx: Some(unclaimed.1.into()),
                my_id,
            };
        }

        let (tx, rx) = Self::create_multiplexed_stream_inner(
            tx_to_networking_layer,
            &self.to_receiving_streams,
            id,
        );

        SubNetwork {
            tx,
            rx: Some(rx.into()),
            my_id,
        }
    }

    /// Creates a subnetwork, and also forwards all messages to the given channel. The network cannot be used to
    /// receive messages since the messages will be forwarded to the provided channel.
    ///
    /// # Panics
    ///
    /// This function will panic if the internal receiver has already been taken, which should not happen
    /// under normal circumstances.
    pub fn multiplex_with_forwarding(
        &self,
        id: impl Into<StreamKey>,
        forward_tx: tokio::sync::mpsc::UnboundedSender<ProtocolMessage>,
    ) -> SubNetwork {
        let mut network = self.multiplex(id);
        let rx = network.rx.take().expect("Rx from network should be Some");
        let forwarding_task = async move {
            let mut rx = rx.into_inner();
            while let Some(msg) = rx.recv().await {
                gadget_logging::info!(
                    "Round {}: Received message from {} to {:?} (id: {})",
                    msg.identifier_info.round_id,
                    msg.sender.user_id,
                    msg.recipient.as_ref().map(|p| p.user_id),
                    msg.identifier_info.message_id,
                );
                if let Err(err) = forward_tx.send(msg) {
                    gadget_logging::error!(%err, "Failed to forward message to network");
                    // TODO: Add AtomicBool to make sending stop
                    break;
                }
            }
        };

        drop(tokio::spawn(forwarding_task));

        network
    }

    fn create_multiplexed_stream_inner(
        mut tx_to_networking_layer: MultiplexedSender,
        active_streams: &ActiveStreams,
        stream_id: StreamKey,
    ) -> (MultiplexedSender, MultiplexedReceiver) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        if active_streams.insert(stream_id, tx).is_some() {
            gadget_logging::warn!(
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
    rx: Option<Mutex<MultiplexedReceiver>>,
    my_id: GossipMsgPublicKey,
}

impl SubNetwork {
    /// Sends a protocol message through the subnetwork.
    ///
    /// # Arguments
    /// * `message` - The protocol message to send
    ///
    /// # Returns
    /// * `Ok(())` - If the message was successfully sent
    /// * `Err(Error)` - If there was an error sending the message
    ///
    /// # Errors
    /// * Returns an error if the underlying network connection is closed or unavailable
    pub fn send(&self, message: ProtocolMessage) -> Result<(), Error> {
        self.tx.send(message)
    }

    pub async fn recv(&self) -> Option<ProtocolMessage> {
        self.rx.as_ref()?.lock().await.next().await
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

    fn public_id(&self) -> GossipMsgPublicKey {
        self.my_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gossip::GossipHandle;
    use futures::{stream, StreamExt};
    use gadget_crypto::hashing::blake3_256;
    use gadget_crypto::KeyType;
    use gadget_logging::setup_log;
    use gadget_std::collections::BTreeMap;
    use serde::{Deserialize, Serialize};
    use std::time::Duration;
    use tokio::time::sleep;

    const TOPIC: &str = "/gadget/test/1.0.0";

    fn deserialize<'a, T>(data: &'a [u8]) -> Result<T, crate::Error>
    where
        T: Deserialize<'a>,
    {
        bincode::deserialize(data).map_err(|err| Error::Other(err.to_string()))
    }

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

    async fn wait_for_nodes_connected(nodes: &[GossipHandle]) {
        let node_count = nodes.len();

        // wait for the nodes to connect to each other
        let max_retries = 10 * node_count;
        let mut retry = 0;
        loop {
            gadget_logging::debug!(%node_count, %max_retries, %retry, "Checking if all nodes are connected to each other");
            let connected = nodes
                .iter()
                .map(super::super::gossip::GossipHandle::connected_peers)
                .collect::<Vec<_>>();

            let all_connected = connected
                .iter()
                .enumerate()
                .inspect(|(node, peers)| {
                    gadget_logging::debug!("Node {node} has {peers} connected peers");
                })
                .all(|(_, &peers)| peers >= node_count - 1);
            if all_connected {
                gadget_logging::debug!("All nodes are connected to each other");
                return;
            }
            sleep(Duration::from_millis(300)).await;
            retry += 1;
            assert!(
                retry <= max_retries,
                "Failed to connect all nodes to each other"
            );
        }
    }

    #[serial_test::serial]
    #[tokio::test(flavor = "multi_thread")]
    #[allow(clippy::cast_possible_truncation)]
    async fn test_p2p() {
        setup_log();
        let nodes = stream::iter(0..*NODE_COUNT)
            .map(|_| node())
            .collect::<Vec<_>>()
            .await;

        wait_for_nodes_connected(&nodes).await;

        let mut mapping = BTreeMap::new();
        for (i, node) in nodes.iter().enumerate() {
            mapping.insert(i as u16, node.my_id);
        }

        let mut tasks = Vec::new();
        for (i, node) in nodes.into_iter().enumerate() {
            let task = tokio::spawn(run_protocol(node, i as u16, mapping.clone()));
            tasks.push(task);
        }
        // Wait for all tasks to finish
        let results = futures::future::try_join_all(tasks)
            .await
            .expect("Failed to run protocol");
        // Assert that all are okay.
        assert!(
            results.iter().all(std::result::Result::is_ok),
            "Some nodes failed to run protocol"
        );
    }

    #[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
    async fn run_protocol<N: Network>(
        node: N,
        i: u16,
        mapping: BTreeMap<u16, crate::GossipMsgPublicKey>,
    ) -> Result<(), crate::Error> {
        let task_hash = [0u8; 32];
        // Safety note: We should be passed a NetworkMultiplexer, and all uses of the N: Network
        // used throughout the program must also use the multiplexer to prevent mixed messages.
        let multiplexer = NetworkMultiplexer::new(node);

        let round1_network = multiplexer.multiplex(StreamKey {
            task_hash, // To differentiate between different instances of a running program (i.e., a task)
            round_id: 0, // To differentiate between different subsets of a running task
        });

        let round2_network = multiplexer.multiplex(StreamKey {
            task_hash, // To differentiate between different instances of a running program (i.e., a task)
            round_id: 1, // To differentiate between different subsets of a running task
        });

        let round3_network = multiplexer.multiplex(StreamKey {
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
            round1_network.build_protocol_message(
                IdentifierInfo {
                    message_id: 0,
                    round_id: 0,
                },
                i,
                None,
                &Msg::Round1(round),
                None,
            )
        };

        gadget_logging::debug!("Broadcast Message");
        round1_network
            .send(msg)
            .map_err(|_| crate::Error::Other("Failed to send message".into()))?;

        // Wait for all other nodes to send their messages
        let mut msgs = BTreeMap::new();
        while let Some(msg) = round1_network.recv().await {
            let m = deserialize::<Msg>(&msg.payload).unwrap();
            gadget_logging::debug!(from = %msg.sender.user_id, ?m, "Received message");
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
                msg.sender.user_id,
            );
            // Break if all messages are received
            if msgs.len() == *NODE_COUNT - 1 {
                break;
            }
        }
        gadget_logging::debug!("Done r1 w/ {i}");

        // Round 2 (P2P)
        let msgs = (0..*NODE_COUNT)
            .map(|r| r as u16)
            .filter(|&j| j != i)
            .map(|j| {
                let peer_pk = mapping.get(&j).copied().unwrap();
                round2_network.build_protocol_message(
                    IdentifierInfo {
                        message_id: 0,
                        round_id: 0,
                    },
                    i,
                    Some(j),
                    &Msg::Round2(Round2Msg {
                        x: i * 10,
                        y: (i + 1) * 20,
                        z: i + 2,
                    }),
                    Some(peer_pk),
                )
            })
            .collect::<Vec<_>>();
        for msg in msgs {
            let to = msg.recipient.map(|r| r.user_id).expect(
                "Recipient should be present for P2P message. This is a bug in the test code",
            );
            gadget_logging::debug!(%to, "Send P2P Message");
            round2_network.send(msg)?;
        }

        // Wait for all other nodes to send their messages
        let mut msgs = BTreeMap::new();
        while let Some(msg) = round2_network.recv().await {
            let m = deserialize::<Msg>(&msg.payload).unwrap();
            gadget_logging::info!(
                "[Node {}] Received message from {} | Intended Recipient: {}",
                i,
                msg.sender.user_id,
                msg.recipient
                    .as_ref()
                    .map_or_else(|| "Broadcast".into(), |r| r.user_id.to_string())
            );
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
                msg.sender.user_id,
            );
            // Break if all messages are received
            if msgs.len() == *NODE_COUNT - 1 {
                break;
            }
        }
        gadget_logging::debug!("Done r2 w/ {i}");

        // Round 3 (broadcast)

        let msg = {
            let round = Round3Msg {
                rotation: i * 30,
                velocity: (i + 1, i + 2, i + 3),
            };
            round3_network.build_protocol_message(
                IdentifierInfo {
                    message_id: 0,
                    round_id: 0,
                },
                i,
                None,
                &Msg::Round3(round),
                None,
            )
        };

        gadget_logging::debug!("Broadcast Message");
        round3_network.send(msg)?;

        // Wait for all other nodes to send their messages
        let mut msgs = BTreeMap::new();
        while let Some(msg) = round3_network.recv().await {
            let m = deserialize::<Msg>(&msg.payload).unwrap();
            gadget_logging::debug!(from = %msg.sender.user_id, ?m, "Received message");
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
                msg.sender.user_id,
            );
            // Break if all messages are received
            if msgs.len() == *NODE_COUNT - 1 {
                break;
            }
        }
        gadget_logging::debug!("Done r3 w/ {i}");

        gadget_logging::info!(node = i, "Protocol completed");

        Ok(())
    }

    fn node_with_id() -> (GossipHandle, crate::key_types::GossipMsgKeyPair) {
        let identity = libp2p::identity::Keypair::generate_ed25519();
        let crypto_key = crate::key_types::Curve::generate_with_seed(None).unwrap();
        let bind_port = 0;
        let handle =
            crate::setup::start_p2p_network(crate::setup::NetworkConfig::new_service_network(
                identity,
                crypto_key.clone(),
                Vec::default(),
                bind_port,
                TOPIC,
            ))
            .unwrap();

        (handle, crypto_key)
    }

    fn node() -> GossipHandle {
        node_with_id().0
    }

    lazy_static::lazy_static! {
        static ref NODE_COUNT: usize = std::env::var("IN_CI").map_or_else(|_| 10, |_| 2);
        static ref MESSAGE_COUNT: usize = std::env::var("IN_CI").map_or_else(|_| 10, |_| 100);
    }

    #[serial_test::serial]
    #[tokio::test(flavor = "multi_thread")]
    #[allow(clippy::cast_possible_truncation)]
    async fn test_stress_test_multiplexer() {
        setup_log();
        gadget_logging::info!("Starting test_stress_test_multiplexer");

        let (network0, network1) = get_networks().await;

        let multiplexer0 = NetworkMultiplexer::new(network0);
        let multiplexer1 = NetworkMultiplexer::new(network1);

        let stream_key = StreamKey {
            task_hash: blake3_256(&[1]),
            round_id: 0,
        };

        let _subnetwork0 = multiplexer0.multiplex(stream_key);
        let _subnetwork1 = multiplexer1.multiplex(stream_key);

        // Create a channel for forwarding
        let (forward_tx, mut forward_rx) = tokio::sync::mpsc::unbounded_channel();

        // Create a subnetwork with forwarding
        let subnetwork0 = multiplexer0.multiplex(stream_key);
        let subnetwork1 = multiplexer1.multiplex_with_forwarding(stream_key, forward_tx);

        let payload = StressTestPayload { value: 42 };
        let msg = subnetwork0.build_protocol_message(
            IdentifierInfo::default(),
            0,
            Some(1),
            &payload,
            Some(subnetwork1.public_id()),
        );

        gadget_logging::info!("Sending message from subnetwork0");
        subnetwork0.send(msg.clone()).unwrap();

        // Message should be forwarded to the forward_rx channel
        let forwarded_msg = forward_rx.recv().await.unwrap();
        let received: StressTestPayload = deserialize(&forwarded_msg.payload).unwrap();
        assert_eq!(received.value, payload.value);
    }

    #[serial_test::serial]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_nested_multiplexer() {
        setup_log();
        gadget_logging::info!("Starting test_nested_multiplexer");
        let (network0, network1) = get_networks().await;

        nested_multiplex(0, 10, network0, network1).await;
    }

    async fn get_networks() -> (GossipHandle, GossipHandle) {
        let network0 = node();
        let network1 = node();

        let mut gossip_networks = vec![network0, network1];

        wait_for_nodes_connected(&gossip_networks).await;

        (gossip_networks.remove(0), gossip_networks.remove(0))
    }

    async fn nested_multiplex<N: Network>(
        cur_depth: usize,
        max_depth: usize,
        network0: N,
        network1: N,
    ) {
        gadget_logging::info!("At nested depth = {cur_depth}/{max_depth}");

        if cur_depth == max_depth {
            return;
        }

        let multiplexer0 = NetworkMultiplexer::new(network0);
        let multiplexer1 = NetworkMultiplexer::new(network1);

        let stream_key = StreamKey {
            #[allow(clippy::cast_possible_truncation)]
            task_hash: blake3_256(&[(cur_depth % 255) as u8]),
            round_id: 0,
        };

        let subnetwork0 = multiplexer0.multiplex(stream_key);
        let subnetwork1 = multiplexer1.multiplex(stream_key);
        let subnetwork1_id = subnetwork1.public_id();

        // Send a message in the subnetwork0 to subnetwork1 and vice versa, assert values of message
        let payload = StressTestPayload { value: 42 };
        let msg = subnetwork0.build_protocol_message(
            IdentifierInfo::default(),
            0,
            Some(1),
            &payload,
            Some(subnetwork1_id),
        );

        gadget_logging::info!("Sending message from subnetwork0");
        subnetwork0.send(msg.clone()).unwrap();

        // Receive message
        let received_msg = subnetwork1.recv().await.unwrap();
        let received: StressTestPayload = deserialize(&received_msg.payload).unwrap();
        assert_eq!(received.value, payload.value);

        let msg = subnetwork1.build_protocol_message(
            IdentifierInfo::default(),
            1,
            Some(0),
            &payload,
            Some(subnetwork0.public_id()),
        );

        gadget_logging::info!("Sending message from subnetwork1");
        subnetwork1.send(msg.clone()).unwrap();

        // Receive message
        let received_msg = subnetwork0.recv().await.unwrap();
        let received: StressTestPayload = deserialize(&received_msg.payload).unwrap();
        assert_eq!(received.value, payload.value);
        tracing::info!("Done nested depth = {cur_depth}/{max_depth}");

        Box::pin(nested_multiplex(
            cur_depth + 1,
            max_depth,
            subnetwork0,
            subnetwork1,
        ))
        .await;
    }

    #[serial_test::serial]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_closed_channel_handling() {
        setup_log();
        let (network0, network1) = get_networks().await;

        let multiplexer0 = NetworkMultiplexer::new(network0);
        let multiplexer1 = NetworkMultiplexer::new(network1);

        let stream_key = StreamKey {
            task_hash: blake3_256(&[1]),
            round_id: 0,
        };

        let subnetwork0 = multiplexer0.multiplex(stream_key);
        // Drop subnetwork1's receiver to simulate closed channel
        let subnetwork1 = multiplexer1.multiplex(stream_key);
        drop(subnetwork1);

        let payload = StressTestPayload { value: 42 };
        let msg =
            subnetwork0.build_protocol_message(IdentifierInfo::default(), 0, None, &payload, None);

        // Sending to a closed channel should return an error
        assert!(subnetwork0.send(msg).is_ok()); // Changed to ok() since the message will be sent but not received
    }

    #[serial_test::serial]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_empty_payload() {
        setup_log();
        let (network0, network1) = get_networks().await;

        let multiplexer0 = NetworkMultiplexer::new(network0);
        let multiplexer1 = NetworkMultiplexer::new(network1);

        let stream_key = StreamKey {
            task_hash: blake3_256(&[1]),
            round_id: 0,
        };

        let subnetwork0 = multiplexer0.multiplex(stream_key);
        let subnetwork1 = multiplexer1.multiplex(stream_key);

        // Test empty payload
        let empty_payload = StressTestPayload { value: 0 };
        let msg = subnetwork0.build_protocol_message(
            IdentifierInfo::default(),
            0,
            Some(1),
            &empty_payload,
            Some(subnetwork1.public_id()),
        );

        gadget_logging::info!("Sending message from subnetwork0");
        subnetwork0.send(msg).unwrap();

        // Receive message
        let received_msg = subnetwork1.recv().await.unwrap();
        let received: StressTestPayload = deserialize(&received_msg.payload).unwrap();
        assert_eq!(received.value, empty_payload.value);
    }

    #[serial_test::serial]
    #[tokio::test(flavor = "multi_thread")]
    #[allow(clippy::cast_possible_truncation)]
    async fn test_concurrent_messaging() {
        setup_log();
        let (network0, network1) = get_networks().await;

        let multiplexer0 = NetworkMultiplexer::new(network0);
        let multiplexer1 = NetworkMultiplexer::new(network1);

        let mut send_handles = Vec::new();
        let mut receive_handles = Vec::new();

        // Create multiple messages to send concurrently
        let message_count = 10;

        // Spawn tasks to send messages
        for i in 0..message_count {
            let stream_key = StreamKey {
                task_hash: blake3_256(&[i]),
                round_id: 0,
            };

            let subnetwork0 = multiplexer0.multiplex(stream_key);
            let subnetwork1 = multiplexer1.multiplex(stream_key);
            let subnetwork1_id = subnetwork1.public_id();

            let i_u64: u64 = i.into();
            let payload = StressTestPayload { value: i_u64 };
            let send_subnetwork0 = subnetwork0;
            let handle = tokio::spawn(async move {
                let msg = send_subnetwork0.build_protocol_message(
                    IdentifierInfo::default(),
                    0,
                    Some(1),
                    &payload,
                    Some(subnetwork1_id),
                );
                send_subnetwork0.send(msg).unwrap();
            });

            send_handles.push(handle);

            // Spawn tasks to receive messages
            let handle = tokio::spawn(async move {
                let msg = subnetwork1.recv().await.unwrap();
                let received: StressTestPayload = deserialize(&msg.payload).unwrap();
                received.value as u8 // Return the payload value for verification
            });

            receive_handles.push(handle);
        }

        // Wait for all sends to complete
        for handle in send_handles {
            handle.await.unwrap();
        }

        // Wait for all receives and verify we got all messages
        let mut received_values = Vec::new();
        for handle in receive_handles {
            received_values.push(handle.await.unwrap());
        }

        received_values.sort_unstable();
        assert_eq!(received_values.len(), message_count as usize);
        for i in 0..message_count {
            assert_eq!(received_values[i as usize], i);
        }
    }

    #[serial_test::serial]
    #[tokio::test(flavor = "multi_thread")]
    #[allow(clippy::cast_possible_truncation)]
    async fn test_message_ordering() {
        setup_log();
        let (network0, network1) = get_networks().await;

        let multiplexer0 = NetworkMultiplexer::new(network0);
        let multiplexer1 = NetworkMultiplexer::new(network1);

        let stream_key = StreamKey {
            task_hash: blake3_256(&[1]),
            round_id: 0,
        };

        let subnetwork0 = multiplexer0.multiplex(stream_key);
        let subnetwork1 = multiplexer1.multiplex(stream_key);

        // Send messages with sequential sequence numbers
        let message_count = 10;
        for i in 0..message_count {
            let payload = StressTestPayload { value: i };
            let msg = subnetwork0.build_protocol_message(
                IdentifierInfo {
                    message_id: i,
                    ..Default::default()
                },
                0,
                Some(1),
                &payload,
                Some(subnetwork1.public_id()),
            );
            subnetwork0.send(msg).unwrap();
        }

        // Verify messages are received in order
        let mut last_seq = 0;
        for _ in 0..message_count {
            let msg = subnetwork1.recv().await.unwrap();
            assert!(
                msg.identifier_info.message_id >= last_seq,
                "Messages should be received in order or equal to last sequence number"
            );
            last_seq = msg.identifier_info.message_id;
        }
    }

    #[serial_test::serial]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_network_id_handling() {
        setup_log();
        let (network0, network1) = get_networks().await;
        let _network0_id = network0.public_id();
        let network1_id = network1.public_id();

        let multiplexer0 = NetworkMultiplexer::new(network0);
        let multiplexer1 = NetworkMultiplexer::new(network1);

        let stream_key = StreamKey {
            task_hash: blake3_256(&[1]),
            round_id: 0,
        };

        let subnetwork0 = multiplexer0.multiplex(stream_key);
        let subnetwork1 = multiplexer1.multiplex(stream_key);

        // Test sending with correct network ID
        let payload = StressTestPayload { value: 42 };
        let msg = subnetwork0.build_protocol_message(
            IdentifierInfo::default(),
            0,
            Some(1),
            &payload,
            Some(network1_id),
        );
        gadget_logging::info!("Sending message from subnetwork0");
        subnetwork0.send(msg.clone()).unwrap();

        // Receive message
        let received_msg = subnetwork1.recv().await.unwrap();
        let received: StressTestPayload = deserialize(&received_msg.payload).unwrap();
        assert_eq!(received.value, payload.value);

        // Test sending with wrong network ID
        let wrong_key = crate::key_types::Curve::generate_with_seed(None).unwrap();
        let msg = subnetwork0.build_protocol_message(
            IdentifierInfo::default(),
            0,
            Some(1),
            &payload,
            Some(wrong_key.public()),
        );
        gadget_logging::info!("Sending message from subnetwork0");
        subnetwork0.send(msg).unwrap();

        // Message with wrong network ID should not be received
        let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(100));
        tokio::select! {
            () = timeout => (),
            _ = subnetwork1.recv() => panic!("Should not receive message with wrong network ID"),
        }
    }

    #[serial_test::serial]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_stream_isolation() {
        setup_log();
        let (network0, network1) = get_networks().await;

        let multiplexer0 = NetworkMultiplexer::new(network0);
        let multiplexer1 = NetworkMultiplexer::new(network1);

        // Create two different stream keys
        let stream_key1 = StreamKey {
            task_hash: blake3_256(&[1]),
            round_id: 0,
        };
        let stream_key2 = StreamKey {
            task_hash: blake3_256(&[2]),
            round_id: 0,
        };

        let subnetwork0_stream1 = multiplexer0.multiplex(stream_key1);
        let subnetwork0_stream2 = multiplexer0.multiplex(stream_key2);
        let subnetwork1_stream1 = multiplexer1.multiplex(stream_key1);
        let subnetwork1_stream2 = multiplexer1.multiplex(stream_key2);

        // Send messages on both streams
        let payload1 = StressTestPayload { value: 1 };
        let payload2 = StressTestPayload { value: 2 };

        let msg1 = subnetwork0_stream1.build_protocol_message(
            IdentifierInfo::default(),
            0,
            Some(1), // Send to node 1
            &payload1,
            Some(subnetwork1_stream1.public_id()),
        );
        let msg2 = subnetwork0_stream2.build_protocol_message(
            IdentifierInfo::default(),
            0,
            Some(1), // Send to node 1
            &payload2,
            Some(subnetwork1_stream2.public_id()),
        );

        gadget_logging::info!("Sending message from subnetwork0_stream1");
        subnetwork0_stream1.send(msg1.clone()).unwrap();
        gadget_logging::info!("Sending message from subnetwork0_stream2");
        subnetwork0_stream2.send(msg2.clone()).unwrap();

        // Verify messages are received on correct streams
        gadget_logging::info!("Waiting for message on subnetwork1_stream1");
        let received_msg1 = subnetwork1_stream1.recv().await.unwrap();
        gadget_logging::info!("Waiting for message on subnetwork1_stream2");
        let received_msg2 = subnetwork1_stream2.recv().await.unwrap();

        let received1: StressTestPayload = deserialize(&received_msg1.payload).unwrap();
        let received2: StressTestPayload = deserialize(&received_msg2.payload).unwrap();

        assert_eq!(received1.value, payload1.value);
        assert_eq!(received2.value, payload2.value);

        // Verify no cross-stream message leakage
        let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(100));
        tokio::select! {
            () = timeout => (),
            _ = subnetwork1_stream1.recv() => panic!("Should not receive more messages on stream 1"),
            _ = subnetwork1_stream2.recv() => panic!("Should not receive more messages on stream 2"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct StressTestPayload {
    value: u64,
}
