use core::pin::Pin;
use core::sync::atomic::AtomicU64;
use core::task::{ready, Context, Poll};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::Arc;

use crate::futures::prelude::*;
use crate::network::{self, IdentifierInfo, Network, NetworkMultiplexer, StreamKey, SubNetwork};
use crate::subxt_core::ext::sp_core::ecdsa;
use round_based::{Delivery, Incoming, Outgoing};
use round_based::{MessageDestination, MessageType, MsgId, PartyIndex};
use stream::{SplitSink, SplitStream};

pub struct NetworkDeliveryWrapper<N, M> {
    /// The wrapped network implementation.
    network: NetworkWrapper<N, M>,
}

impl<N, M> NetworkDeliveryWrapper<N, M>
where
    N: Network + Unpin,
    M: Clone + Send + Unpin + 'static,
    M: serde::Serialize,
    M: serde::de::DeserializeOwned,
{
    /// Create a new NetworkDeliveryWrapper over a network implementation with the given party index.
    pub fn new(
        network: N,
        i: PartyIndex,
        task_hash: [u8; 32],
        parties: BTreeMap<PartyIndex, ecdsa::Public>,
    ) -> Self {
        let mux = NetworkMultiplexer::new(network);
        // By default, we create 10 substreams for each party.
        let sub_streams = (0..10)
            .map(|i| {
                let key = StreamKey {
                    // This is a dummy task hash, it should be replaced with the actual task hash
                    task_hash: [0u8; 32],
                    round_id: i,
                };
                let substream = mux.multiplex(key);
                (key, substream)
            })
            .collect();
        let network = NetworkWrapper {
            me: i,
            mux,
            incoming_queue: VecDeque::new(),
            outgoing_queue: VecDeque::new(),
            sub_streams,
            participants: parties,
            task_hash,
            next_msg_id: Arc::new(NextMessageId::default()),
            _network: core::marker::PhantomData,
        };
        NetworkDeliveryWrapper { network }
    }
}

/// A NetworkWrapper wraps a network implementation and implements [`Stream`] and [`Sink`] for
/// it.
pub struct NetworkWrapper<N, M> {
    /// The current party index.
    me: PartyIndex,
    /// Our network Multiplexer.
    mux: NetworkMultiplexer,
    /// A Map of substreams for each round.
    sub_streams: HashMap<StreamKey, SubNetwork>,
    /// A queue of incoming messages.
    incoming_queue: VecDeque<Incoming<M>>,
    /// A queue of outgoing messages.
    outgoing_queue: VecDeque<Outgoing<M>>,
    /// Participants in the network with their corresponding ECDSA public keys.
    // Note: This is a BTreeMap to ensure that the participants are sorted by their party index.
    participants: BTreeMap<PartyIndex, ecdsa::Public>,
    next_msg_id: Arc<NextMessageId>,
    task_hash: [u8; 32],
    _network: core::marker::PhantomData<N>,
}

impl<N, M> Delivery<M> for NetworkDeliveryWrapper<N, M>
where
    N: Network + Unpin,
    M: Clone + Send + Unpin + 'static,
    M: serde::Serialize + serde::de::DeserializeOwned,
    M: round_based::ProtocolMessage,
{
    type Send = SplitSink<NetworkWrapper<N, M>, Outgoing<M>>;
    type Receive = SplitStream<NetworkWrapper<N, M>>;
    type SendError = crate::Error;
    type ReceiveError = crate::Error;

    fn split(self) -> (Self::Receive, Self::Send) {
        let (sink, stream) = self.network.split();
        (stream, sink)
    }
}

impl<N, M> Stream for NetworkWrapper<N, M>
where
    N: Network + Unpin,
    M: serde::de::DeserializeOwned + Unpin,
    M: round_based::ProtocolMessage,
{
    type Item = Result<Incoming<M>, crate::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let sub_streams = self.sub_streams.values();
        // pull all substreams
        let mut messages = Vec::new();
        for sub_stream in sub_streams {
            let p = sub_stream.next_message().poll_unpin(cx);
            let m = match p {
                Poll::Ready(Some(msg)) => msg,
                _ => continue,
            };
            let msg = network::deserialize::<M>(&m.payload)?;
            messages.push((m.sender.user_id, m.recipient, msg));
        }

        // Sort the incoming messages by round.
        messages.sort_by_key(|(_, _, msg)| msg.round());

        let this = self.get_mut();
        // Push all messages to the incoming queue
        messages
            .into_iter()
            .map(|(sender, recipient, msg)| Incoming {
                id: this.next_msg_id.next(),
                sender,
                msg_type: match recipient {
                    Some(_) => MessageType::P2P,
                    None => MessageType::Broadcast,
                },
                msg,
            })
            .for_each(|m| this.incoming_queue.push_back(m));
        // Reorder the incoming queue by round message.
        let maybe_msg = this.incoming_queue.pop_front();
        if let Some(msg) = maybe_msg {
            Poll::Ready(Some(Ok(msg)))
        } else {
            // No message in the queue, and no message in the substreams.
            // Tell the network to wake us up when a new message arrives.
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

impl<N, M> Sink<Outgoing<M>> for NetworkWrapper<N, M>
where
    N: Network + Unpin,
    M: Unpin + serde::Serialize,
    M: round_based::ProtocolMessage,
{
    type Error = crate::Error;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, msg: Outgoing<M>) -> Result<(), Self::Error> {
        self.get_mut().outgoing_queue.push_back(msg);
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        // Dequeue all messages and send them one by one to the network
        let this = self.get_mut();
        while let Some(out) = this.outgoing_queue.pop_front() {
            // Get the substream to send the message to.
            let key = StreamKey {
                task_hash: [0u8; 32], // TODO: Use real hash
                round_id: i32::from(out.msg.round()),
            };
            let substream = this
                .sub_streams
                .entry(key)
                .or_insert_with(|| this.mux.multiplex(key));
            let identifier_info = IdentifierInfo {
                block_id: None,
                session_id: None,
                retry_id: None,
                task_id: None,
            };
            let (to, to_network_id) = match out.recipient {
                MessageDestination::AllParties => (None, None),
                MessageDestination::OneParty(p) => (Some(p), this.participants.get(&p).cloned()),
            };
            let protocol_message = N::build_protocol_message(
                identifier_info,
                this.me,
                to,
                &out.msg,
                this.participants.get(&this.me).cloned(),
                to_network_id,
            );
            let p = substream.send_message(protocol_message).poll_unpin(cx);
            match ready!(p) {
                Ok(()) => continue,
                Err(e) => return Poll::Ready(Err(e)),
            }
        }
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

#[derive(Default)]
struct NextMessageId(AtomicU64);

impl NextMessageId {
    pub fn next(&self) -> MsgId {
        self.0.fetch_add(1, core::sync::atomic::Ordering::Relaxed)
    }
}
