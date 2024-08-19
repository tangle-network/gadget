use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::{Debug, Display};
use core::hash::Hash;
use core::ops::{Add, Sub};
use serde::{Deserialize, Serialize};

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

/// The [`ProtocolRemote`] is the interface between the [`ProtocolWorkManager`] and the async protocol.
/// It *must* be unique between each async protocol.
pub trait ProtocolRemote<WM: WorkManagerInterface>: Send + Sync + 'static {
    fn start(&self) -> Result<(), WM::Error>;
    fn session_id(&self) -> WM::SessionID;
    fn started_at(&self) -> WM::Clock;
    fn shutdown(&self, reason: ShutdownReason) -> Result<(), WM::Error>;
    fn is_done(&self) -> bool;
    fn deliver_message(&self, message: WM::ProtocolMessage) -> Result<(), WM::Error>;
    fn has_started(&self) -> bool;
    fn retry_id(&self) -> WM::RetryID;

    fn has_stalled(&self, now: WM::Clock) -> bool {
        now >= self.started_at() + WM::acceptable_block_tolerance()
    }

    fn is_active(&self) -> bool {
        // If the protocol has started, is not done, and has not stalled, then it is active
        self.has_started() && !self.is_done() && !self.has_started()
    }
}
