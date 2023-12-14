//! When delivering messages to an async protocol, we want o make sure we don't mix up voting and public key gossip messages
//! Thus, this file contains a function that takes a channel from the gadget to the async protocol and splits it into two channels
use futures::StreamExt;
use gadget_common::gadget::message::{GadgetProtocolMessage, UserID};
use gadget_common::gadget::network::Network;
use gadget_common::gadget::work_manager::WebbWorkManager;
use gadget_core::job_manager::WorkManagerInterface;
use round_based::Msg;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

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
    pub id: sp_core::ecdsa::Public,
}

pub fn create_job_manager_to_async_protocol_channel_split<
    N: Network + 'static,
    C1: Serialize + DeserializeOwned + HasSenderAndReceiver + Send + 'static,
    C2: Serialize + DeserializeOwned + HasSenderAndReceiver + Send + 'static,
>(
    mut rx_gadget: UnboundedReceiver<GadgetProtocolMessage>,
    associated_block_id: <WebbWorkManager as WorkManagerInterface>::Clock,
    associated_retry_id: <WebbWorkManager as WorkManagerInterface>::RetryID,
    associated_session_id: <WebbWorkManager as WorkManagerInterface>::SessionID,
    associated_task_id: <WebbWorkManager as WorkManagerInterface>::TaskID,
    network: N,
) -> (
    futures::channel::mpsc::UnboundedSender<C1>,
    futures::channel::mpsc::UnboundedReceiver<std::io::Result<C1>>,
    UnboundedSender<C2>,
    UnboundedReceiver<C2>,
) {
    let (tx_to_async_proto_1, rx_for_async_proto_1) = futures::channel::mpsc::unbounded();
    let (tx_to_async_proto_2, rx_for_async_proto_2) = tokio::sync::mpsc::unbounded_channel();
    // Take the messages from the gadget and send them to the async protocol
    tokio::task::spawn(async move {
        while let Some(msg) = rx_gadget.recv().await {
            match bincode2::deserialize::<SplitChannelMessage<C1, C2>>(&msg.payload) {
                Ok(msg) => match msg {
                    SplitChannelMessage::Channel1(msg) => {
                        if tx_to_async_proto_1.unbounded_send(Ok(msg)).is_err() {
                            log::error!("Failed to send message to protocol");
                        }
                    }
                    SplitChannelMessage::Channel2(msg) => {
                        if tx_to_async_proto_2.send(msg).is_err() {
                            log::error!("Failed to send message to protocol");
                        }
                    }
                },
                Err(err) => {
                    log::error!("Failed to deserialize message: {err:?}");
                }
            }
        }
    });

    let (tx_to_outbound_1, mut rx_to_outbound_1) = futures::channel::mpsc::unbounded::<C1>();
    let (tx_to_outbound_2, mut rx_to_outbound_2) = tokio::sync::mpsc::unbounded_channel::<C2>();
    let network_clone = network.clone();
    // Take the messages the async protocol sends to the outbound channel and send them to the gadget
    tokio::task::spawn(async move {
        let offline_task = async move {
            while let Some(msg) = rx_to_outbound_1.next().await {
                let from = msg.sender();
                let to = msg.receiver();
                let msg = SplitChannelMessage::<C1, C2>::Channel1(msg);
                let msg = GadgetProtocolMessage {
                    associated_block_id,
                    associated_session_id,
                    associated_retry_id,
                    task_hash: associated_task_id,
                    from,
                    to,
                    payload: bincode2::serialize(&msg).expect("Failed to serialize message"),
                    from_network_id: None, // TODO: Mapping of [task_hash] => UserID => AccountID for mapping userIDs to accountIDs
                    to_network_id: None, // TODO: Mapping of [task_hash] => UserID => AccountID for mapping userIDs to accountIDs
                };

                if network.send_message(msg).await.is_err() {
                    log::error!("Failed to send message to outbound");
                }
            }
        };

        let voting_task = async move {
            while let Some(msg) = rx_to_outbound_2.recv().await {
                let from = msg.sender();
                let to = msg.receiver();
                let msg = SplitChannelMessage::<C1, C2>::Channel2(msg);
                let msg = GadgetProtocolMessage {
                    associated_block_id,
                    associated_session_id,
                    associated_retry_id,
                    task_hash: associated_task_id,
                    from,
                    to,
                    payload: bincode2::serialize(&msg).expect("Failed to serialize message"),
                    from_network_id: None, // TODO: Mapping of [task_hash] => UserID => AccountID for mapping userIDs to accountIDs
                    to_network_id: None, // TODO: Mapping of [task_hash] => UserID => AccountID for mapping userIDs to accountIDs
                };

                if network_clone.send_message(msg).await.is_err() {
                    log::error!("Failed to send message to outbound");
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

pub trait HasSenderAndReceiver {
    fn sender(&self) -> UserID;
    fn receiver(&self) -> Option<UserID>;
}

impl<T> HasSenderAndReceiver for Msg<T> {
    fn sender(&self) -> UserID {
        self.sender as UserID
    }

    fn receiver(&self) -> Option<UserID> {
        self.receiver.map(|r| r as UserID)
    }
}

impl HasSenderAndReceiver for VotingMessage {
    fn sender(&self) -> UserID {
        self.from
    }

    fn receiver(&self) -> Option<UserID> {
        self.to
    }
}

impl HasSenderAndReceiver for PublicKeyGossipMessage {
    fn sender(&self) -> UserID {
        self.from
    }

    fn receiver(&self) -> Option<UserID> {
        self.to
    }
}
