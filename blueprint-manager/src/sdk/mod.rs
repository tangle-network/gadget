pub mod config;
pub mod entry;
pub mod keystore;
pub mod setup;
pub mod utils;

pub use config::*;
pub use entry::run_gadget_for_protocol;
pub use gadget_common::prelude::*;
pub use gadget_core::gadget::general::Client;
pub use gadget_sdk::network::setup::{start_p2p_network, NetworkConfig};
pub use setup::generate_node_input;

pub mod prelude {
    pub use crate::sdk::async_trait;
    pub use crate::sdk::BuiltExecutableJobWrapper;
    pub use crate::sdk::DebugLogger;
    pub use crate::sdk::ECDSAKeyStore;
    pub use crate::sdk::FullProtocolConfig;
    pub use crate::sdk::JobError;
    pub use crate::sdk::JobsClient;
    pub use crate::sdk::KeystoreBackend;
    pub use crate::sdk::Mutex;
    pub use crate::sdk::Network;
    pub use crate::sdk::NodeInput;
    pub use crate::sdk::ProtocolWorkManager;
    pub use crate::sdk::SendFuture;
    pub use crate::sdk::UnboundedReceiver;
    pub use crate::sdk::WorkManagerInterface;
    pub use gadget_common::full_protocol::SharedOptional;

    pub use gadget_common::prelude::InMemoryBackend;
    pub use gadget_common::tangle_runtime::AccountId32;
    pub use gadget_common::Error;
    pub use gadget_common::{generate_protocol, generate_setup_and_run_command};
    pub use std::sync::Arc;

    pub use color_eyre;
    pub use tangle_primitives;
    pub use tangle_subxt;
}
