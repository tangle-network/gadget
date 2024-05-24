use crate::client::ClientWithApi;
use crate::gadget::message::UserID;
use crate::gadget::tangle::runtime::TangleRuntime;
use crate::gadget::tangle::TangleEvent;
use crate::prelude::{TangleProtocolMessage, TangleWorkManager};
use crate::utils::serialize;
use gadget_core::job_manager::WorkManagerInterface;
use serde::Serialize;
use sp_core::ecdsa;
use std::error::Error;

pub trait GadgetEnvironment {
    type Event: Send + Sync + 'static;
    type ProtocolMessage: Send + Sync + 'static;
    type Client: ClientWithApi<Self::Event> + Send + Sync + 'static;
    type JobManager: WorkManagerInterface;
    type Error: Error + Send + Sync + 'static;

    fn build_protocol_message<Payload: Serialize>(
        associated_block_id: <Self::JobManager as WorkManagerInterface>::Clock,
        associated_session_id: <Self::JobManager as WorkManagerInterface>::SessionID,
        associated_retry_id: <Self::JobManager as WorkManagerInterface>::RetryID,
        associated_task_id: <Self::JobManager as WorkManagerInterface>::TaskID,
        from: UserID,
        to: Option<UserID>,
        payload: &Payload,
        from_account_id: Option<ecdsa::Public>,
        to_network_id: Option<ecdsa::Public>,
    ) -> Self::ProtocolMessage;
}

#[derive(Default)]
pub struct TangleEnvironment;

impl GadgetEnvironment for TangleEnvironment {
    type Event = TangleEvent;
    type ProtocolMessage = TangleProtocolMessage;
    type Client = TangleRuntime;
    type JobManager = TangleWorkManager;
    type Error = crate::Error;

    fn build_protocol_message<Payload: Serialize>(
        associated_block_id: <Self::JobManager as WorkManagerInterface>::Clock,
        associated_session_id: <Self::JobManager as WorkManagerInterface>::SessionID,
        associated_retry_id: <Self::JobManager as WorkManagerInterface>::RetryID,
        associated_task_id: <Self::JobManager as WorkManagerInterface>::TaskID,
        from: UserID,
        to: Option<UserID>,
        payload: &Payload,
        from_account_id: Option<ecdsa::Public>,
        to_network_id: Option<ecdsa::Public>,
    ) -> Self::ProtocolMessage {
        TangleProtocolMessage {
            associated_block_id,
            associated_session_id,
            associated_retry_id,
            task_hash: associated_task_id,
            from,
            to: to.as_user_id(),
            payload: serialize(payload).expect("Failed to serialize message"),
            from_network_id: from_account_id,
            to_network_id,
        }
    }
}
