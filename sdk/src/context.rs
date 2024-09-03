use alloc::boxed::Box;
use alloc::fmt::{Debug, Display};
use alloc::string::String;
use alloc::vec::Vec;
use async_trait::async_trait;
use core::error::Error;
use core::hash::Hash;
use core::ops::{Add, Sub};
use gadget_core::gadget::general::Client;
use gadget_core::job::protocol::{ProtocolMessageMetadata, WorkManagerInterface};
use serde::{Deserialize, Serialize};
use sp_core::ecdsa;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum PollMethod {
    Interval { millis: u64 },
    Manual,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum DeliveryType {
    EnqueuedProtocol,
    ActiveProtocol,
    // Protocol for the message is not yet available
    EnqueuedMessage,
}

#[derive(Debug)]
pub enum ShutdownReason {
    Stalled,
    DropCode,
}

pub trait WorkManagerInterface: Send + Sync + 'static + Sized {
    type RetryID: Copy + Hash + Eq + PartialEq + Send + Sync + 'static;
    type Clock: Copy
        + Debug
        + Default
        + Eq
        + Ord
        + PartialOrd
        + PartialEq
        + Send
        + Sync
        + Sub<Output = Self::Clock>
        + Add<Output = Self::Clock>
        + 'static;
    type ProtocolMessage: Serialize
        + for<'de> Deserialize<'de>
        + ProtocolMessageMetadata<Self>
        + Send
        + Sync
        + Clone
        + 'static;
    type Error: Debug + Send + Sync + 'static;
    type SessionID: Copy + Hash + Eq + PartialEq + Display + Debug + Send + Sync + 'static;
    type TaskID: Copy + Hash + Eq + PartialEq + Debug + Send + Sync + AsRef<[u8]> + 'static;

    fn debug(&self, input: String);
    fn error(&self, input: String);
    fn warn(&self, input: String);
    fn clock(&self) -> Self::Clock;
    fn acceptable_block_tolerance() -> Self::Clock;
    fn associated_block_id_acceptable(expected: Self::Clock, received: Self::Clock) -> bool {
        // Favor explicit logic for readability
        let tolerance = Self::acceptable_block_tolerance();
        let is_acceptable_above = received >= expected && received <= expected + tolerance;
        let is_acceptable_below =
            received < expected && received >= saturating_sub(expected, tolerance);
        let is_equal = expected == received;

        is_acceptable_above || is_acceptable_below || is_equal
    }
}

fn saturating_sub<T: Sub<Output = T> + Ord + Default>(a: T, b: T) -> T {
    if a < b {
        T::default()
    } else {
        a - b
    }
}

pub trait ProtocolMessageMetadata<WM: WorkManagerInterface> {
    fn associated_block_id(&self) -> WM::Clock;
    fn associated_session_id(&self) -> WM::SessionID;
    fn associated_retry_id(&self) -> WM::RetryID;
    fn associated_task(&self) -> WM::TaskID;
    fn associated_sender_user_id(&self) -> u16;
    fn associated_recipient_user_id(&self) -> Option<u16>;
    fn payload(&self) -> &Vec<u8>;
    fn sender_network_id(&self) -> Option<sp_core::ecdsa::Public>;
    fn recipient_network_id(&self) -> Option<sp_core::ecdsa::Public>;
    fn payload_mut(&mut self) -> &mut Vec<u8>;
}

#[async_trait]
pub trait GadgetContext: Debug + Sized + 'static
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
    type ProtocolMessage: Serialize
        + for<'de> Deserialize<'de>
        + Send
        + Sync
        + 'static
        + ProtocolMessageMetadata<Self::WorkManager>;
    type Client: Client<<Self as GadgetContext>::Event> + Send + Sync + 'static;
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

    async fn setup_runtime(&self) -> Result<Self::Client, Self::Error>;

    fn transaction_manager(&self) -> Self::TransactionManager;

    fn set_payload(&mut self, input: Vec<u8>, output: &mut Vec<u8>) {
        *output = input;
    }
}
