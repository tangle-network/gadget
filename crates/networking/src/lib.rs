#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

pub mod gossip;
pub mod handlers;
pub mod messaging;
pub mod networking;
#[cfg(feature = "round-based-compat")]
pub mod round_based_compat;
pub mod setup;

use gadget_std::string::String;

/// Re-exported networking crates
#[cfg(feature = "round-based-compat")]
pub use round_based;

/// Unique identifier for a party
pub type UserID = u16;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Channel error: {0}")]
    ChannelError(String),

    #[error("Gossip error: {0}")]
    GossipError(String),

    #[error("Messaging error: {0}")]
    MessagingError(String),

    #[error("Round based error: {0}")]
    RoundBasedError(String),

    #[error("Serde JSON error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("Other error: {0}")]
    Other(String),
}
