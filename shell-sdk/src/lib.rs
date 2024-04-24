pub mod config;
pub mod entry;
pub mod keystore;
pub mod network;
pub mod shell;
pub mod tangle;

pub use config::*;
pub use entry::run_shell_for_protocol;
pub use gadget_common::prelude::*;
pub use gadget_core::gadget::substrate::Client;
pub use shell::generate_node_input;

/// Should be put inside the main.rs file of the protocol repository
macro_rules! generate_shell_binary {
    ($entry_point:expr, $n_protocols:expr, $( $role_type:expr ),*) => {
        #[tokio::main]
        async fn main {
            $crate::run_shell_for_protocol(vec![$($role_type),*], $n_protocols, $entry_point).await?;
        }
    };
}
