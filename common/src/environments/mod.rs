use crate::client::ClientWithApi;
use async_trait::async_trait;
use gadget_core::job_manager::{ProtocolMessageMetadata, WorkManagerInterface};
use serde::{Deserialize, Serialize};
use sp_core::ecdsa;
use std::error::Error;
use std::fmt::{Debug, Display};

pub trait EventMetadata<Env: GadgetEnvironment> {
    fn number(&self) -> <Env as GadgetEnvironment>::Clock;
}

#[async_trait]
pub trait GadgetEnvironment: std::fmt::Debug + Sized + 'static
where
    Self::WorkManager: WorkManagerInterface<
        Clock = Self::Clock,
        RetryID = Self::RetryID,
        TaskID = Self::TaskID,
        SessionID = Self::SessionID,
        Error = Self::Error,
        ProtocolMessage = Self::ProtocolMessage,
    >,
{
    type Event: EventMetadata<Self> + Send + Sync + 'static;
    type ProtocolMessage: Serialize
        + for<'de> Deserialize<'de>
        + Send
        + Sync
        + 'static
        + ProtocolMessageMetadata<Self::WorkManager>;
    type Client: ClientWithApi<Self> + Send + Sync + 'static;
    type WorkManager: WorkManagerInterface;
    type Error: Error + Send + Sync + From<String> + Into<crate::Error> + 'static;
    type Clock: Display + Copy + Send + Sync + 'static;
    type RetryID: Display + Copy + Send + Sync + 'static;
    type TaskID: Debug + Copy + Send + Sync + 'static;
    type SessionID: Display + Copy + Send + Sync + 'static;
    type TransactionManager: Clone + Send + Sync + 'static;
    type JobInitMetadata: Send + Sync + 'static;

    #[allow(clippy::too_many_arguments)]
    fn build_protocol_message<Payload: Serialize>(
        associated_block_id: <Self::WorkManager as WorkManagerInterface>::Clock,
        associated_session_id: <Self::WorkManager as WorkManagerInterface>::SessionID,
        associated_retry_id: <Self::WorkManager as WorkManagerInterface>::RetryID,
        associated_task_id: <Self::WorkManager as WorkManagerInterface>::TaskID,
        from: u16,
        to: Option<u16>,
        payload: &Payload,
        from_account_id: Option<ecdsa::Public>,
        to_network_id: Option<ecdsa::Public>,
    ) -> Self::ProtocolMessage;

    async fn setup_client(&self) -> Result<Self::Client, Self::Error>;

    fn transaction_manager(&self) -> Self::TransactionManager;

    fn set_payload(&mut self, input: Vec<u8>, output: &mut Vec<u8>) {
        *output = input;
    }
}
