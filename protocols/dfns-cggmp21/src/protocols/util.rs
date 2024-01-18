//! When delivering messages to an async protocol, we want o make sure we don't mix up voting and public key gossip messages
//! Thus, this file contains a function that takes a channel from the gadget to the async protocol and splits it into two channels
use dfns_cggmp21::round_based::{Incoming, MessageDestination, MessageType, Outgoing, PartyIndex};
use futures::StreamExt;
use gadget_common::client::AccountId;
use gadget_common::gadget::message::{GadgetProtocolMessage, UserID};
use gadget_common::gadget::network::Network;
use gadget_common::gadget::work_manager::WorkManager;
use gadget_core::job_manager::WorkManagerInterface;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;

#[derive(Serialize, Deserialize, Debug)]
pub enum SplitChannelMessage<C1, C2> {
    Channel1(C1),
    Channel2(C2),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VotingMessage {
    pub from: UserID,
    pub to: Option<UserID>,
    pub payload: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PublicKeyGossipMessage {
    pub from: UserID,
    pub to: Option<UserID>,
    pub signature: Vec<u8>,
    pub id: AccountId,
}

/// All possible senders of a message
#[derive(Debug, Default, Serialize, Deserialize)]
enum MaybeSender {
    /// We are the sender of the message
    Myself,
    /// The sender is someone else
    /// it could also be us, double check the [`UserID`]
    SomeoneElse(UserID),
    /// The sender is unknown.
    #[default]
    Unknown,
}

impl MaybeSender {
    /// Returns `true` if the maybe sender is [`Myself`].
    ///
    /// [`Myself`]: MaybeSender::Myself
    #[must_use]
    fn is_myself(&self) -> bool {
        matches!(self, Self::Myself)
    }

    /// Returns `true` if the maybe sender is [`Myself`].
    /// Or if the sender is [`SomeoneElse`] but the [`UserID`] is the same as `my_user_id`
    ///
    /// [`Myself`]: MaybeSender::Myself
    /// [`SomeoneElse`]: MaybeSender::SomeoneElse
    #[must_use]
    fn is_myself_check(&self, my_user_id: UserID) -> bool {
        match self {
            Self::Myself => true,
            Self::SomeoneElse(id) if (*id == my_user_id) => true,
            _ => false,
        }
    }

    /// Returns `true` if the maybe sender is [`SomeoneElse`].
    ///
    /// [`SomeoneElse`]: MaybeSender::SomeoneElse
    #[must_use]
    fn is_someone_else(&self) -> bool {
        matches!(self, Self::SomeoneElse(..))
    }

    /// Returns `true` if the maybe sender is [`Unknown`].
    ///
    /// [`Unknown`]: MaybeSender::Unknown
    #[must_use]
    fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }

    /// Returns the sender as [`UserID`] if it is knwon.
    #[must_use]
    fn as_user_id(&self) -> Option<UserID> {
        match self {
            Self::Myself => None,
            Self::SomeoneElse(id) => Some(*id),
            Self::Unknown => None,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
enum MaybeReceiver {
    /// The message is broadcasted to everyone
    Broadcast,
    /// The message is sent to a specific party
    P2P(UserID),
    /// The receiver is us.
    Myself,
    /// The receiver is unknown.
    #[default]
    Unknown,
}

impl MaybeReceiver {
    /// Returns `true` if the maybe receiver is [`Broadcast`].
    ///
    /// [`Broadcast`]: MaybeReceiver::Broadcast
    #[must_use]
    fn is_broadcast(&self) -> bool {
        matches!(self, Self::Broadcast)
    }

    /// Returns `true` if the maybe receiver is [`P2P`].
    ///
    /// [`P2P`]: MaybeReceiver::P2P
    #[must_use]
    fn is_p2_p(&self) -> bool {
        matches!(self, Self::P2P(..))
    }

    /// Returns `true` if the maybe receiver is [`Myself`].
    ///
    /// [`Myself`]: MaybeReceiver::Myself
    #[must_use]
    fn is_myself(&self) -> bool {
        matches!(self, Self::Myself)
    }

    /// Returns `true` if the maybe receiver is [`Myself`]
    /// Or if the receiver is [`P2P`] but the [`UserID`] is the same as `my_user_id`
    ///
    /// [`Myself`]: MaybeReceiver::Myself
    /// [`P2P`]: MaybeReceiver::P2P
    #[must_use]
    fn is_myself_check(&self, my_user_id: UserID) -> bool {
        match self {
            Self::Myself => true,
            Self::P2P(id) if (*id == my_user_id) => true,
            _ => false,
        }
    }

    /// Returns `true` if the maybe receiver is [`Unknown`].
    ///
    /// [`Unknown`]: MaybeReceiver::Unknown
    #[must_use]
    fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }

    /// Returns the receiver as [`UserID`] if it is knwon.
    fn as_user_id(&self) -> Option<UserID> {
        match self {
            Self::Broadcast => None,
            Self::P2P(id) => Some(*id),
            Self::Myself => None,
            Self::Unknown => None,
        }
    }
}

/// A Simple trait to extract the sender and the receiver from a message
trait MaybeSenderReceiver {
    fn maybe_sender(&self) -> MaybeSender;
    fn maybe_receiver(&self) -> MaybeReceiver;
}

impl MaybeSenderReceiver for PublicKeyGossipMessage {
    fn maybe_sender(&self) -> MaybeSender {
        MaybeSender::SomeoneElse(self.from)
    }
    fn maybe_receiver(&self) -> MaybeReceiver {
        match self.to {
            Some(id) => MaybeReceiver::P2P(id),
            None => MaybeReceiver::Broadcast,
        }
    }
}

impl MaybeSenderReceiver for VotingMessage {
    fn maybe_sender(&self) -> MaybeSender {
        MaybeSender::SomeoneElse(self.from)
    }
    fn maybe_receiver(&self) -> MaybeReceiver {
        match self.to {
            Some(id) => MaybeReceiver::P2P(id),
            None => MaybeReceiver::Broadcast,
        }
    }
}

impl<M> MaybeSenderReceiver for Outgoing<M> {
    fn maybe_sender(&self) -> MaybeSender {
        MaybeSender::Myself
    }

    fn maybe_receiver(&self) -> MaybeReceiver {
        match self.recipient {
            MessageDestination::AllParties => MaybeReceiver::Broadcast,
            MessageDestination::OneParty(i) => MaybeReceiver::P2P(i as UserID),
        }
    }
}

impl<M> MaybeSenderReceiver for Incoming<M> {
    fn maybe_sender(&self) -> MaybeSender {
        MaybeSender::SomeoneElse(self.sender as UserID)
    }

    fn maybe_receiver(&self) -> MaybeReceiver {
        match self.msg_type {
            MessageType::Broadcast => MaybeReceiver::Broadcast,
            MessageType::P2P => MaybeReceiver::Myself,
        }
    }
}

impl MaybeSenderReceiver for () {
    fn maybe_sender(&self) -> MaybeSender {
        MaybeSender::Unknown
    }

    fn maybe_receiver(&self) -> MaybeReceiver {
        MaybeReceiver::Unknown
    }
}

pub fn create_job_manager_to_async_protocol_channel_split<
    N: Network + 'static,
    C2: Serialize + DeserializeOwned + MaybeSenderReceiver + Send + 'static,
    M: Serialize + DeserializeOwned + Send + 'static,
>(
    mut rx_gadget: UnboundedReceiver<GadgetProtocolMessage>,
    associated_block_id: <WorkManager as WorkManagerInterface>::Clock,
    associated_retry_id: <WorkManager as WorkManagerInterface>::RetryID,
    associated_session_id: <WorkManager as WorkManagerInterface>::SessionID,
    associated_task_id: <WorkManager as WorkManagerInterface>::TaskID,
    user_id_mapping: Arc<HashMap<UserID, AccountId>>,
    my_account_id: AccountId,
    network: N,
) -> (
    futures::channel::mpsc::UnboundedSender<Outgoing<M>>,
    futures::channel::mpsc::UnboundedReceiver<
        Result<Incoming<M>, futures::channel::mpsc::TryRecvError>,
    >,
    futures::channel::mpsc::UnboundedSender<C2>,
    futures::channel::mpsc::UnboundedReceiver<C2>,
) {
    let (tx_to_async_proto_1, rx_for_async_proto_1) = futures::channel::mpsc::unbounded();
    let (tx_to_async_proto_2, rx_for_async_proto_2) = futures::channel::mpsc::unbounded();

    // Take the messages from the gadget and send them to the async protocol
    tokio::task::spawn(async move {
        while let Some(msg_orig) = rx_gadget.recv().await {
            match bincode2::deserialize::<SplitChannelMessage<M, C2>>(&msg_orig.payload) {
                Ok(msg) => match msg {
                    SplitChannelMessage::Channel1(msg) => {
                        let msg_type = if msg_orig.to.is_some() {
                            MessageType::P2P
                        } else {
                            MessageType::Broadcast
                        };
                        let incoming = Incoming {
                            id: 0, // TODO: How to get this value??
                            sender: msg_orig.from as PartyIndex,
                            msg_type,
                            msg,
                        };
                        if tx_to_async_proto_1.unbounded_send(Ok(incoming)).is_err() {
                            log::error!(target: "gadget", "Failed to send message to protocol");
                        }
                    }
                    SplitChannelMessage::Channel2(msg) => {
                        if tx_to_async_proto_2.unbounded_send(msg).is_err() {
                            log::error!(target: "gadget", "Failed to send message to protocol");
                        }
                    }
                },
                Err(err) => {
                    log::error!(target: "gadget", "Failed to deserialize message: {err:?}");
                }
            }
        }
    });

    let (tx_to_outbound_1, mut rx_to_outbound_1) =
        futures::channel::mpsc::unbounded::<Outgoing<M>>();
    let (tx_to_outbound_2, mut rx_to_outbound_2) = futures::channel::mpsc::unbounded::<C2>();
    let network_clone = network.clone();
    let user_id_mapping_clone = user_id_mapping.clone();
    let my_user_id = user_id_mapping
        .iter()
        .find_map(|(user_id, account_id)| {
            if *account_id == my_account_id {
                Some(*user_id)
            } else {
                None
            }
        })
        .expect("Failed to find my user id");
    // Take the messages from the async protocol and send them to the gadget
    tokio::task::spawn(async move {
        let offline_task = async move {
            while let Some(msg) = rx_to_outbound_1.next().await {
                let from = msg.maybe_sender();
                let to = msg.maybe_receiver();
                let (to_account_id, from_account_id) = get_to_and_from_account_id(
                    &user_id_mapping_clone,
                    from.as_user_id().unwrap_or(my_user_id),
                    to.as_user_id(),
                );
                let msg = SplitChannelMessage::<M, C2>::Channel1(msg.msg);
                let msg = GadgetProtocolMessage {
                    associated_block_id,
                    associated_session_id,
                    associated_retry_id,
                    task_hash: associated_task_id,
                    from: from.as_user_id().unwrap_or(my_user_id),
                    to: to.as_user_id(),
                    payload: bincode2::serialize(&msg).expect("Failed to serialize message"),
                    from_network_id: from_account_id,
                    to_network_id: to_account_id,
                };

                if let Err(err) = network.send_message(msg).await {
                    log::error!(target:"gadget", "Failed to send message to outbound: {err:?}");
                }
            }
        };

        let voting_task = async move {
            while let Some(msg) = rx_to_outbound_2.next().await {
                let from = msg.maybe_sender();
                let to = msg.maybe_receiver();
                let (to_account_id, from_account_id) = get_to_and_from_account_id(
                    &user_id_mapping,
                    from.as_user_id().unwrap_or(my_user_id),
                    to.as_user_id(),
                );
                let msg = SplitChannelMessage::<M, C2>::Channel2(msg);
                let msg = GadgetProtocolMessage {
                    associated_block_id,
                    associated_session_id,
                    associated_retry_id,
                    task_hash: associated_task_id,
                    from: from.as_user_id().unwrap_or(my_user_id),
                    to: to.as_user_id(),
                    payload: bincode2::serialize(&msg).expect("Failed to serialize message"),
                    from_network_id: from_account_id,
                    to_network_id: to_account_id,
                };

                if let Err(err) = network_clone.send_message(msg).await {
                    log::error!(target:"gadget", "Failed to send message to outbound: {err:?}");
                }
            }
        };

        tokio::join!(offline_task, voting_task);
    });

    (
        tx_to_outbound_1,
        rx_for_async_proto_1,
        tx_to_outbound_2,
        rx_for_async_proto_2,
    )
}

fn get_to_and_from_account_id(
    mapping: &HashMap<UserID, AccountId>,
    from: UserID,
    to: Option<UserID>,
) -> (Option<AccountId>, Option<AccountId>) {
    let from_account_id = mapping.get(&from).cloned();
    let to_account_id = if let Some(to) = to {
        mapping.get(&to).cloned()
    } else {
        None
    };

    (to_account_id, from_account_id)
}
