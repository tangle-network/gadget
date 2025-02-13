#[cfg(test)]
mod tests;

use crate::key_types::GossipMsgPublicKey;
use crate::Error;
use async_trait::async_trait;
use dashmap::DashMap;
use futures::{Stream, StreamExt};
use gadget_crypto::hashing::blake3_256;
use gadget_std as std;
use gadget_std::boxed::Box;
use gadget_std::cmp::Reverse;
use gadget_std::collections::{BinaryHeap, HashMap};
use gadget_std::fmt::Display;
use gadget_std::format;
use gadget_std::ops::{Deref, DerefMut};
use gadget_std::pin::Pin;
use gadget_std::string::ToString;
use gadget_std::sync::Arc;
use gadget_std::task::{Context, Poll};
use gadget_std::vec::Vec;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;

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
    sequence_number: u64,
    payload: Vec<u8>,
}

#[derive(Debug)]
struct PendingMessage {
    sequence_number: u64,
    message: ProtocolMessage,
}

impl PartialEq for PendingMessage {
    fn eq(&self, other: &Self) -> bool {
        self.sequence_number == other.sequence_number
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
        self.sequence_number.cmp(&other.sequence_number)
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
    pub my_id: GossipMsgPublicKey,
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

                    gadget_logging::trace!(
                        "SEND SEQ {current_seq} FROM {} | StreamKey: {:?}",
                        msg.sender.user_id,
                        hex::encode(bincode::serialize(&compound_key).unwrap())
                    );

                    let multiplexed_message = MultiplexedMessage {
                        stream_id,
                        payload: SequencedMessage {
                            sequence_number: current_seq,
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

                    let Ok(multiplexed_message) = bincode::deserialize::<MultiplexedMessage>(&msg.payload) else {
                        gadget_logging::error!("Failed to deserialize message (networking)");
                        continue;
                    };

                    let stream_id = multiplexed_message.stream_id;
                    let compound_key = CompoundStreamKey {
                        stream_key: stream_id,
                        send_user: msg.sender.user_id,
                        recv_user: msg.recipient.as_ref().map(|p| p.user_id),
                    };
                    let seq = multiplexed_message.payload.sequence_number;
                    msg.payload = multiplexed_message.payload.payload;

                    // Get or create the pending heap for this stream
                    let pending = pending_messages.entry(compound_key).or_default();
                    let expected_seq = expected_seqs.entry(compound_key).or_default();

                    let send_user = msg.sender.user_id;
                    let recv_user = msg.recipient.as_ref().map(|p| p.user_id);
                    let compound_key_hex =
                        hex::encode(bincode::serialize(&compound_key).unwrap());
                    gadget_logging::trace!(
                        "RECV SEQ {seq} FROM {} as user {:?} | Expecting: {} | StreamKey: {:?}",
                        send_user,
                        recv_user,
                        *expected_seq,
                        compound_key_hex,
                    );

                    // Add the message to pending
                    pending.push(Reverse(PendingMessage { sequence_number: seq, message: msg }));

                    // Try to deliver messages in order
                    if let Some(active_receiver) = active_streams.get(&stream_id) {
                        while let Some(Reverse(PendingMessage { sequence_number, message: _ })) =
                            pending.peek()
                        {
                            if *sequence_number != *expected_seq {
                                gadget_logging::error!("Sequence number mismatch, expected {} but got {}", *expected_seq, sequence_number);
                                break;
                            }

                            gadget_logging::trace!("DELIVERING SEQ {seq} FROM {} as user {:?} | Expecting: {} | StreamKey: {:?}", send_user, recv_user, *expected_seq, compound_key_hex);

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
                        while let Some(Reverse(PendingMessage { sequence_number, message: _ })) =
                            pending.peek()
                        {
                            if *sequence_number != *expected_seq {
                                gadget_logging::error!("Sequence number mismatch, expected {} but got {}", *expected_seq, sequence_number);
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
