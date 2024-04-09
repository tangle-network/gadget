pub mod shared;
#[cfg(not(target_family = "wasm"))]
pub mod standard;
#[cfg(target_family = "wasm")]
pub mod wasm;

#[cfg(target_family = "wasm")]
pub use wasm::{
    keystore::{KeystoreConfig, SubstrateKeystore},
};

#[cfg(not(target_family = "wasm"))]
pub use standard::{
    keystore::{KeystoreConfig, SubstrateKeystore},
};

#[cfg(target_family = "wasm")]
pub use tokio;
// pub use tokio_wasm as tokio;
#[cfg(not(target_family = "wasm"))]
pub use tokio;