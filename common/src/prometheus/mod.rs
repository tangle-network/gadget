mod shared;
#[cfg(not(target_family = "wasm"))]
mod standard;
#[cfg(target_family = "wasm")]
mod wasm;

pub use shared::*;
#[cfg(not(target_family = "wasm"))]
pub use standard::*; //{setup, PrometheusConfig, BYTES_RECEIVED, BYTES_SENT, REGISTRY};
#[cfg(target_family = "wasm")]
pub use wasm::*; //{setup, PrometheusConfig, BYTES_RECEIVED, BYTES_SENT};
