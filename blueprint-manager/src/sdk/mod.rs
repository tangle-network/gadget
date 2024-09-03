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
    pub use color_eyre;
    pub use std::sync::Arc;
    pub use tangle_primitives;
    pub use tangle_subxt;
}
