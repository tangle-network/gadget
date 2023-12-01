use gadget_core::job_manager::WorkManagerInterface;
use round_based::{Msg, StateMachine};
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
