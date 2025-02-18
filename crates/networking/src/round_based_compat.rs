use crate::key_types::GossipMsgPublicKey;
use crate::networking::{NetworkMultiplexer, StreamKey, SubNetwork};
use crate::types::{IdentifierInfo, ParticipantInfo, ProtocolMessage};
use core::pin::Pin;
use core::sync::atomic::AtomicU64;
use core::task::{ready, Context, Poll};
use futures::prelude::*;
use gadget_std::collections::{BTreeMap, HashMap};
use gadget_std::string::ToString;
use gadget_std::sync::Arc;
use round_based::{Delivery, Incoming, MessageType, Outgoing};
use round_based::{MessageDestination, MsgId, PartyIndex};
use stream::{SplitSink, SplitStream};

pub struct NetworkDeliveryWrapper<M> {
    /// The wrapped network implementation.
    network: NetworkWrapper<M>,
}

impl<M> NetworkDeliveryWrapper<M>
where
    M: Clone + Send + Unpin + 'static,
    M: serde::Serialize + serde::de::DeserializeOwned,
{
    /// Create a new `NetworkDeliveryWrapper` over a network implementation with the given party index.
    #[must_use]
    pub fn new<const N: usize>(
        mux: Arc<NetworkMultiplexer>,
        i: PartyIndex,
        task_hash: [u8; 32],
        parties: BTreeMap<PartyIndex, GossipMsgPublicKey>,
    ) -> Self {
        let (tx_forward, rx) = tokio::sync::mpsc::unbounded_channel();
        // By default, we create 10 substreams for each party.
        let mut sub_streams = HashMap::new();
        for x in 0..N {
            let key = StreamKey {
                task_hash,
                round_id: x as i32,
            };
            // Creates a multiplexed subnetwork, and also forwards all messages to the given channel
            let _ = sub_streams.insert(key, mux.multiplex_with_forwarding(key, tx_forward.clone()));
        }

        let network = NetworkWrapper {
            me: i,
            mux,
            message_hashes: HashMap::new(),
            sub_streams,
            participants: parties,
            task_hash,
            tx_forward,
            rx,
            next_msg_id: Arc::new(NextMessageId::default()),
            _phantom: std::marker::PhantomData,
        };

        NetworkDeliveryWrapper { network }
    }
}

/// A `NetworkWrapper` wraps a network implementation
/// and implements [`Stream`] and [`Sink`] for it.
pub struct NetworkWrapper<M> {
    /// The current party index.
    me: PartyIndex,
    /// Our network Multiplexer.
    mux: Arc<NetworkMultiplexer>,
    /// A Map of substreams for each round.
    sub_streams: HashMap<StreamKey, SubNetwork>,
    /// A map of message hashes to their corresponding message id.
    /// This is used to deduplicate messages.
    message_hashes: HashMap<blake3::Hash, MsgId>,
    /// Participants in the network with their corresponding public keys.
    /// Note: This is a `BTreeMap` to ensure that the participants are sorted by their party index.
    participants: BTreeMap<PartyIndex, GossipMsgPublicKey>,
    /// The next message id to use.
    next_msg_id: Arc<NextMessageId>,
    /// A channel for forwarding messages to the network.
    tx_forward: tokio::sync::mpsc::UnboundedSender<ProtocolMessage>,
    /// A channel for receiving messages from the network.
    rx: tokio::sync::mpsc::UnboundedReceiver<ProtocolMessage>,
    /// The task hash of the current task.
    task_hash: [u8; 32],
    /// A phantom data type to ensure that the network wrapper is generic over the message type.
    _phantom: std::marker::PhantomData<M>,
}

impl<M> Delivery<M> for NetworkDeliveryWrapper<M>
where
    M: Clone + Send + Unpin + 'static,
    M: serde::Serialize + serde::de::DeserializeOwned,
    M: round_based::ProtocolMessage,
{
    type Send = SplitSink<NetworkWrapper<M>, Outgoing<M>>;
    type Receive = SplitStream<NetworkWrapper<M>>;
    type SendError = crate::error::Error;
    type ReceiveError = crate::error::Error;

    fn split(self) -> (Self::Receive, Self::Send) {
        let (sink, stream) = self.network.split();
        (stream, sink)
    }
}

impl<M> Stream for NetworkWrapper<M>
where
    M: serde::de::DeserializeOwned + Unpin,
    M: round_based::ProtocolMessage,
{
    type Item = Result<Incoming<M>, crate::error::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let res = ready!(this.rx.poll_recv(cx));
        if let Some(res) = res {
            let msg_type = if res.recipient.is_some() {
                MessageType::P2P
            } else {
                MessageType::Broadcast
            };

            let id = res.identifier_info.message_id;

            let msg: M = match serde_json::from_slice(&res.payload) {
                Ok(msg) => msg,
                Err(err) => {
                    gadget_logging::error!(%err, "Failed to deserialize message (round_based_compat)");
                    return Poll::Ready(Some(Err(crate::error::Error::Other(err.to_string()))));
                }
            };

            let message_hash = blake3::hash(&res.payload);
            gadget_logging::debug!(
                "Received message with hash {} from {} in round {}",
                hex::encode(message_hash.as_bytes()),
                res.sender.user_id,
                res.identifier_info.round_id
            );

            if this.message_hashes.contains_key(&message_hash) {
                gadget_logging::warn!(
                    "Received duplicate message with hash {} (id: {})",
                    hex::encode(message_hash.as_bytes()),
                    id
                );
                return Poll::Ready(None);
            }

            this.message_hashes.insert(message_hash, id);

            Poll::Ready(Some(Ok(Incoming {
                msg,
                sender: res.sender.user_id,
                id,
                msg_type,
            })))
        } else {
            Poll::Ready(None)
        }
    }
}

impl<M> Sink<Outgoing<M>> for NetworkWrapper<M>
where
    M: Unpin + serde::Serialize,
    M: round_based::ProtocolMessage,
{
    type Error = crate::error::Error;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, out: Outgoing<M>) -> Result<(), Self::Error> {
        let this = self.get_mut();
        let id = this.next_msg_id.next();

        let round_id = out.msg.round();

        gadget_logging::info!(
            "Round {}: Sending message from {} to {:?} (id: {})",
            round_id,
            this.me,
            out.recipient,
            id,
        );

        // Get the substream to send the message to.
        let key = StreamKey {
            task_hash: this.task_hash,
            round_id: i32::from(round_id),
        };
        let substream = this.sub_streams.entry(key).or_insert_with(|| {
            this.mux
                .multiplex_with_forwarding(key, this.tx_forward.clone())
        });

        let identifier_info = IdentifierInfo {
            message_id: id,
            round_id,
        };
        let (to, to_network_id) = match out.recipient {
            MessageDestination::AllParties => (None, None),
            MessageDestination::OneParty(p) => (Some(p), this.participants.get(&p).copied()),
        };

        if matches!(out.recipient, MessageDestination::OneParty(_)) && to_network_id.is_none() {
            gadget_logging::warn!("Recipient not found when required for {:?}", out.recipient);
            return Err(crate::error::Error::Other(
                "Recipient not found".to_string(),
            ));
        }

        // Manually construct a `ProtocolMessage` since rounds-based
        // does not work well with bincode
        let protocol_message = ProtocolMessage {
            identifier_info,
            sender: ParticipantInfo {
                user_id: this.me,
                public_key: this.participants.get(&this.me).copied(),
            },
            recipient: to.map(|user_id| ParticipantInfo {
                user_id,
                public_key: to_network_id,
            }),
            payload: serde_json::to_vec(&out.msg).expect("Should be able to serialize message"),
        };

        match substream.send(protocol_message) {
            Ok(()) => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

#[derive(Default)]
struct NextMessageId(AtomicU64);

impl NextMessageId {
    fn next(&self) -> MsgId {
        self.0
            .fetch_add(1, gadget_std::sync::atomic::Ordering::Relaxed)
    }
}
