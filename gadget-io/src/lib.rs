pub mod shared;
#[cfg(not(target_family = "wasm"))]
pub mod standard;
#[cfg(target_family = "wasm")]
pub mod wasm;

#[cfg(target_family = "wasm")]
pub use wasm::{
    keystore::{KeystoreConfig, SubstrateKeystore, KeystoreContainer},
    shell::{TomlConfig, Opt, SupportedChains}
};

#[cfg(not(target_family = "wasm"))]
pub use standard::{
    keystore::{KeystoreConfig, SubstrateKeystore, KeystoreContainer},
    shell::{TomlConfig, Opt, SupportedChains, defaults}
};

#[cfg(target_family = "wasm")]
pub use tokio;
// pub use tokio_wasm as tokio;
#[cfg(not(target_family = "wasm"))]
pub use tokio;

#[cfg(target_family = "wasm")]
use wasm_bindgen::{
    prelude::*,
    JsCast
};

#[cfg(target_family = "wasm")]
use wasm_bindgen_futures;

#[cfg(target_family = "wasm")]
pub fn into_js_error(err: impl std::error::Error) -> JsValue {
    js_sys::Error::new(&err.to_string()).into()
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);
}