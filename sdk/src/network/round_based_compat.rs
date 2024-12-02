use core::pin::Pin;
use core::sync::atomic::AtomicU64;
use core::task::{ready, Context, Poll};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::Arc;

use crate::futures::prelude::*;
use crate::network::{IdentifierInfo, NetworkMultiplexer, ProtocolMessage, StreamKey, SubNetwork};
use crate::subxt_core::ext::sp_core::ecdsa;
use round_based::{Delivery, Incoming, MessageType, Outgoing};
use round_based::{MessageDestination, MsgId, PartyIndex};
use stream::{SplitSink, SplitStream};

use super::ParticipantInfo;

pub struct NetworkDeliveryWrapper<M> {
    /// The wrapped network implementation.
    network: NetworkWrapper<M>,
}

impl<M> NetworkDeliveryWrapper<M>
where
    M: Clone + Send + Unpin + 'static,
    M: serde::Serialize + serde::de::DeserializeOwned,
{
    /// Create a new NetworkDeliveryWrapper over a network implementation with the given party index.
    pub fn new(
        mux: Arc<NetworkMultiplexer>,
        i: PartyIndex,
        task_hash: [u8; 32],
        parties: BTreeMap<PartyIndex, ecdsa::Public>,
    ) -> Self {
        let (tx_forward, rx) = tokio::sync::mpsc::unbounded_channel();
        // By default, we create 10 substreams for each party.
        let mut sub_streams = HashMap::new();
        for x in 0..10 {
            let key = StreamKey {
                task_hash,
                round_id: x,
            };
            // Creates a multiplexed subnetwork, and also forwards all messages to the given channel
            let _ = sub_streams.insert(key, mux.multiplex_with_forwarding(key, tx_forward.clone()));
        }

        let network = NetworkWrapper {
            me: i,
            mux,
            incoming_queue: VecDeque::new(),
            sub_streams,
            participants: parties,
            task_hash,
            tx_forward,
            rx,
            next_msg_id: Arc::new(NextMessageId::default()),
        };

        NetworkDeliveryWrapper { network }
    }
}

/// A NetworkWrapper wraps a network implementation and implements [`Stream`] and [`Sink`] for
/// it.
pub struct NetworkWrapper<M> {
    /// The current party index.
    me: PartyIndex,
    /// Our network Multiplexer.
    mux: Arc<NetworkMultiplexer>,
    /// A Map of substreams for each round.
    sub_streams: HashMap<StreamKey, SubNetwork>, //HashMap<StreamKey, SubNetwork>,
    /// A queue of incoming messages.
    #[allow(dead_code)]
    incoming_queue: VecDeque<Incoming<M>>,
    /// Participants in the network with their corresponding ECDSA public keys.
    // Note: This is a BTreeMap to ensure that the participants are sorted by their party index.
    participants: BTreeMap<PartyIndex, ecdsa::Public>,
    next_msg_id: Arc<NextMessageId>,
    tx_forward: tokio::sync::mpsc::UnboundedSender<ProtocolMessage>,
    rx: tokio::sync::mpsc::UnboundedReceiver<ProtocolMessage>,
    task_hash: [u8; 32],
}

impl<M> Delivery<M> for NetworkDeliveryWrapper<M>
where
    M: Clone + Send + Unpin + 'static,
    M: serde::Serialize + serde::de::DeserializeOwned,
    M: round_based::ProtocolMessage,
{
    type Send = SplitSink<NetworkWrapper<M>, Outgoing<M>>;
    type Receive = SplitStream<NetworkWrapper<M>>;
    type SendError = crate::Error;
    type ReceiveError = crate::Error;

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
    type Item = Result<Incoming<M>, crate::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let res = ready!(self.get_mut().rx.poll_recv(cx));
        if let Some(res) = res {
            let msg_type = if res.recipient.is_some() {
                MessageType::P2P
            } else {
                MessageType::Broadcast
            };

            let id = res.identifier_info.message_id;

            let msg = match bincode::deserialize(&res.payload) {
                Ok(msg) => msg,
                Err(err) => {
                    crate::error!(%err, "Failed to deserialize message");
                    return Poll::Ready(Some(Err(crate::Error::Other(err.to_string()))));
                }
            };

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
    type Error = crate::Error;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, out: Outgoing<M>) -> Result<(), Self::Error> {
        let this = self.get_mut();
        let id = this.next_msg_id.next();

        let round_id = out.msg.round();

        crate::info!(
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
            MessageDestination::OneParty(p) => (Some(p), this.participants.get(&p).cloned()),
        };

        if matches!(out.recipient, MessageDestination::OneParty(_)) && to_network_id.is_none() {
            crate::warn!("Recipient not found when required for {:?}", out.recipient);
            return Err(crate::Error::Other("Recipient not found".to_string()));
        }

        let protocol_message = ProtocolMessage {
            identifier_info,
            sender: ParticipantInfo {
                user_id: this.me,
                ecdsa_key: this.participants.get(&this.me).cloned(),
            },
            recipient: to.map(|user_id| ParticipantInfo {
                user_id,
                ecdsa_key: to_network_id,
            }),
            payload: bincode::serialize(&out.msg).expect("Should be able to serialize message"),
        };

        match substream.send(protocol_message) {
            Ok(()) => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

#[derive(Default)]
struct NextMessageId(AtomicU64);

impl NextMessageId {
    fn next(&self) -> MsgId {
        self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }
}
