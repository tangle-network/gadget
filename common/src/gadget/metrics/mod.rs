#[cfg(not(target_family = "wasm"))]
mod standard;
#[cfg(target_family = "wasm")]
mod wasm;

#[cfg(not(target_family = "wasm"))]
pub use standard::Metrics;
#[cfg(target_family = "wasm")]
pub use wasm::Metrics;
