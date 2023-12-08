use futures::StreamExt;
use gadget_core::job_manager::WorkManagerInterface;
use multi_party_ecdsa::gg_2020::state_machine::sign::{OfflineProtocolMessage, OfflineStage};
use round_based::{Msg, StateMachine};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use webb_gadget::gadget::message::{GadgetProtocolMessage, UserID};
use webb_gadget::gadget::network::Network;
use webb_gadget::gadget::work_manager::WebbWorkManager;

pub fn create_job_manager_to_async_protocol_channel<SM: StateMachine, N: Network>(
    mut rx_gadget: UnboundedReceiver<GadgetProtocolMessage>,
    associated_block_id: <WebbWorkManager as WorkManagerInterface>::Clock,
    associated_retry_id: <WebbWorkManager as WorkManagerInterface>::RetryID,
    associated_session_id: <WebbWorkManager as WorkManagerInterface>::SessionID,
    associated_task_id: <WebbWorkManager as WorkManagerInterface>::TaskID,
    network: N,
) -> (
    futures::channel::mpsc::UnboundedSender<Msg<SM::MessageBody>>,
    futures::channel::mpsc::UnboundedReceiver<std::io::Result<Msg<SM::MessageBody>>>,
)
where
    <SM as StateMachine>::MessageBody: Send + Serialize + DeserializeOwned,
{
    let (tx_to_async_proto, rx_for_async_proto) = futures::channel::mpsc::unbounded();
    // Take the messages from the gadget and send them to the async protocol
    tokio::task::spawn(async move {
        while let Some(msg) = rx_gadget.recv().await {
            match bincode2::deserialize::<Msg<SM::MessageBody>>(&msg.payload) {
                Ok(msg) => {
                    if tx_to_async_proto.unbounded_send(Ok(msg)).is_err() {
                        log::error!("Failed to send message to protocol");
                    }
                }
                Err(err) => {
                    log::error!("Failed to deserialize message: {err:?}");
                }
            }
        }
    });

    let (tx_to_outbound, mut rx_to_outbound) =
        futures::channel::mpsc::unbounded::<Msg<SM::MessageBody>>();
    // Take the messages the async protocol sends to the outbound channel and send them to the gadget
    tokio::task::spawn(async move {
        while let Some(msg) = rx_to_outbound.next().await {
            let msg = GadgetProtocolMessage {
                associated_block_id,
                associated_session_id,
                associated_retry_id,
                task_hash: associated_task_id,
                from: msg.sender as UserID,
                to: msg.receiver.map(|r| r as UserID),
                payload: bincode2::serialize(&msg).expect("Failed to serialize message"),
            };

            if network.send_message(msg).await.is_err() {
                log::error!("Failed to send message to outbound");
            }
        }
    });

    (tx_to_outbound, rx_for_async_proto)
}

#[derive(Serialize, Deserialize, Debug)]
pub enum SigningMessage {
    Offline(Msg<<OfflineStage as StateMachine>::MessageBody>),
    Voting(VotingMessage),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VotingMessage {
    pub from: UserID,
    pub to: Option<UserID>,
    pub payload: Vec<u8>,
}

pub fn create_job_manager_to_async_protocol_channel_signing<N: Network>(
    mut rx_gadget: UnboundedReceiver<GadgetProtocolMessage>,
    associated_block_id: <WebbWorkManager as WorkManagerInterface>::Clock,
    associated_retry_id: <WebbWorkManager as WorkManagerInterface>::RetryID,
    associated_session_id: <WebbWorkManager as WorkManagerInterface>::SessionID,
    associated_task_id: <WebbWorkManager as WorkManagerInterface>::TaskID,
    network: N,
) -> (
    futures::channel::mpsc::UnboundedSender<Msg<OfflineProtocolMessage>>,
    futures::channel::mpsc::UnboundedReceiver<std::io::Result<Msg<OfflineProtocolMessage>>>,
    UnboundedSender<VotingMessage>,
    UnboundedReceiver<VotingMessage>,
) {
    let (tx_to_async_proto_offline, rx_for_async_proto_offline) =
        futures::channel::mpsc::unbounded();
    let (tx_to_async_proto_voting, rx_for_async_proto_voting) =
        tokio::sync::mpsc::unbounded_channel();
    // Take the messages from the gadget and send them to the async protocol
    tokio::task::spawn(async move {
        while let Some(msg) = rx_gadget.recv().await {
            match bincode2::deserialize::<SigningMessage>(&msg.payload) {
                Ok(msg) => match msg {
                    SigningMessage::Offline(msg) => {
                        if tx_to_async_proto_offline.unbounded_send(Ok(msg)).is_err() {
                            log::error!("Failed to send message to protocol");
                        }
                    }
                    SigningMessage::Voting(msg) => {
                        if tx_to_async_proto_voting.send(msg).is_err() {
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

    let (tx_to_outbound_offline, mut rx_to_outbound_offline) =
        futures::channel::mpsc::unbounded::<Msg<OfflineProtocolMessage>>();
    let (tx_to_outbound_voting, mut rx_to_outbound_voting) =
        tokio::sync::mpsc::unbounded_channel::<VotingMessage>();
    // Take the messages the async protocol sends to the outbound channel and send them to the gadget
    tokio::task::spawn(async move {
        let offline_task = async move {
            while let Some(msg) = rx_to_outbound_offline.next().await {
                let from = msg.sender as UserID;
                let to = msg.receiver.map(|r| r as UserID);
                let msg = SigningMessage::Offline(msg);
                let msg = GadgetProtocolMessage {
                    associated_block_id,
                    associated_session_id,
                    associated_retry_id,
                    task_hash: associated_task_id,
                    from,
                    to,
                    payload: bincode2::serialize(&msg).expect("Failed to serialize message"),
                };

                if network.send_message(msg).await.is_err() {
                    log::error!("Failed to send message to outbound");
                }
            }
        };

        let voting_task = async move {
            while let Some(msg) = rx_to_outbound_voting.recv().await {
                let from = msg.from;
                let to = msg.to;
                let msg = SigningMessage::Voting(msg);
                let msg = GadgetProtocolMessage {
                    associated_block_id,
                    associated_session_id,
                    associated_retry_id,
                    task_hash: associated_task_id,
                    from,
                    to,
                    payload: bincode2::serialize(&msg).expect("Failed to serialize message"),
                };

                if network.send_message(msg).await.is_err() {
                    log::error!("Failed to send message to outbound");
                }
            }
        };

        tokio::join!(offline_task, voting_task);
    });

    (
        tx_to_outbound_offline,
        rx_for_async_proto_offline,
        tx_to_outbound_voting,
        rx_for_async_proto_voting,
    )
}
