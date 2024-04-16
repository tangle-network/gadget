use color_eyre;
use sp_core::{ecdsa, ed25519, sr25519, Pair, crypto};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures;
use wasm_bindgen::JsValue;

pub trait SubstrateKeystore {
    fn ecdsa_key(&self) -> color_eyre::Result<ecdsa::Pair>;

    fn sr25519_key(&self) -> color_eyre::Result<sr25519::Pair>;
}