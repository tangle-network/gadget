pub mod config;
pub mod entry;
pub mod keystore;
pub mod network;
pub mod shell;
pub mod tangle;

pub use config::*;
pub use entry::run_shell_for_protocol;
pub use gadget_common::prelude::*;
pub use shell::run_forever;

/// Should be put inside the main.rs file of the protocol repository
macro_rules! generate_shell_binary {
    ($n_protocols:expr, $( $role_type:expr ),*) => {
        #[tokio::main]
        async fn main {
            $crate::run_shell_for_protocol(vec![$($role_type),*], $n_protocols).await?;
        }
    };
}
