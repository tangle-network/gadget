use gadget_core::job_manager::WorkManagerInterface;
use multi_party_ecdsa::gg_2020::state_machine::sign::OfflineStage;
use round_based::{Msg, StateMachine};
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
    UnboundedSender<SM::MessageBody>,
    UnboundedReceiver<SM::MessageBody>,
) {
    let (tx_to_async_proto, rx_for_async_proto) = tokio::sync::mpsc::unbounded_channel();
    // Take the messages from the gadget and send them to the async protocol
    tokio::task::spawn(async move {
        while let Some(msg) = rx_gadget.recv().await {
            match bincode2::deserialize::<SM::MessageBody>(&msg.payload) {
                Ok(msg) => {
                    if tx_to_async_proto.send(msg).is_err() {
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
        tokio::sync::mpsc::unbounded_channel::<Msg<SM::MessageBody>>();
    // Take the messages the async protocol sends to the outbound channel and send them to the gadget
    tokio::task::spawn(async move {
        while let Some(msg) = rx_to_outbound.recv().await {
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
    Offline(<OfflineStage as StateMachine>::MessageBody),
    Voting(VotingMessage),
}

pub struct VotingMessage {
    pub from: UserID,
    pub to: Option<UserID>,
    pub payload: Vec<u8>,
}

pub fn create_job_manager_to_async_protocol_channel_signing<SM: StateMachine, N: Network>(
    mut rx_gadget: UnboundedReceiver<GadgetProtocolMessage>,
    associated_block_id: <WebbWorkManager as WorkManagerInterface>::Clock,
    associated_retry_id: <WebbWorkManager as WorkManagerInterface>::RetryID,
    associated_session_id: <WebbWorkManager as WorkManagerInterface>::SessionID,
    associated_task_id: <WebbWorkManager as WorkManagerInterface>::TaskID,
    network: N,
) -> (
    UnboundedSender<Msg<<SM as StateMachine>::MessageBody>>,
    UnboundedReceiver<<SM as StateMachine>::MessageBody>,
    UnboundedSender<VotingMessage>,
    UnboundedReceiver<VotingMessage>,
) {
    let (tx_to_async_proto_offline, rx_for_async_proto_offline) =
        tokio::sync::mpsc::unbounded_channel();
    let (tx_to_async_proto_voting, rx_for_async_proto_voting) =
        tokio::sync::mpsc::unbounded_channel();
    // Take the messages from the gadget and send them to the async protocol
    tokio::task::spawn(async move {
        while let Some(msg) = rx_gadget.recv().await {
            match bincode2::deserialize::<SigningMessage>(&msg.payload) {
                Ok(msg) => match msg {
                    SigningMessage::Offline(msg) => {
                        if tx_to_async_proto_offline.send(msg).is_err() {
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
        tokio::sync::mpsc::unbounded_channel::<Msg<SM::MessageBody>>();
    let (tx_to_outbound_voting, mut rx_to_outbound_voting) =
        tokio::sync::mpsc::unbounded_channel::<VotingMessage>();
    // Take the messages the async protocol sends to the outbound channel and send them to the gadget
    tokio::task::spawn(async move {
        let offline_task = async move {
            while let Some(msg) = rx_to_outbound_offline.recv().await {
                let from = msg.sender as UserID;
                let to = msg.receiver.map(|r| r as UserID);
                let msg = SigningMessage::Offline(msg.body);
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
