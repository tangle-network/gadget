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

pub use pallet_dkg;
pub use pallet_jobs;
pub use pallet_jobs_rpc_runtime_api;
pub use pallet_zksaas;
pub use tangle_primitives;
pub use tangle_subxt;

/// Should be put inside the main.rs file of the protocol repository
#[macro_export]
macro_rules! generate_shell_binary {
    ($entry_point:path, $keystore:path, $n_protocols:expr, $($role_type:expr),*) => {
        #[tokio::main]
        async fn main() -> Result<(), gadget_common::Error> {
            $crate::run_shell_for_protocol(vec![$($role_type),*], $n_protocols, $keystore, $entry_point).await?;
            Ok(())
        }
    };
}
