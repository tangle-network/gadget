mod shared;
pub use shared::*;

pub mod error;
mod imp;

pub use imp::*;

#[cfg(all(feature = "std", target_family = "wasm"))]
pub use {tokio, wasm_bindgen_futures::spawn_local as spawn};

#[cfg(all(feature = "std", not(target_family = "wasm")))]
pub use {tokio, tokio::task::spawn};

#[cfg(all(feature = "std", target_family = "wasm"))]
pub use wasmtimer::tokio as time;

#[cfg(all(feature = "std", not(target_family = "wasm")))]
pub use tokio::time;

#[cfg(all(feature = "std", not(feature = "wasm")))]
pub fn log(s: &str) {
    println!("{}", s);
}

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wasm")]
pub fn into_js_error(err: impl core::error::Error) -> JsValue {
    #[cfg(not(feature = "std"))]
    use alloc::string::ToString;

    js_sys::Error::new(&err.to_string()).into()
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);
}
