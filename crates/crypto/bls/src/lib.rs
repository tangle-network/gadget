#![cfg_attr(not(feature = "std"), no_std)]

pub mod error;
mod w3f_bls;

use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use gadget_std::vec::Vec;
pub use w3f_bls::*;

/// Serialize this to a vector of bytes.
pub fn to_bytes<T: CanonicalSerialize>(elt: T) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(elt.compressed_size());

    <T as CanonicalSerialize>::serialize_compressed(&elt, &mut bytes).unwrap();

    bytes
}

/// Deserialize this from a slice of bytes.
pub fn from_bytes<T: CanonicalDeserialize>(bytes: &[u8]) -> T {
    <T as CanonicalDeserialize>::deserialize_compressed(&mut &bytes[..]).unwrap()
}
