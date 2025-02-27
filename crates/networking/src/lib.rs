#![cfg_attr(docsrs, feature(doc_auto_cfg))]

pub mod behaviours;
pub mod blueprint_protocol;
pub mod discovery;
pub mod error;
pub mod service;
pub mod service_handle;
pub mod types;

#[cfg(test)]
mod tests;

pub use gadget_crypto::KeyType;
pub use service::{AllowedKeys, NetworkConfig, NetworkEvent, NetworkService};
