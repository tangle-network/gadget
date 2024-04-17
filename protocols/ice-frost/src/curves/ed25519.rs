use aes_gcm::Aes128Gcm;

use ice_frost::CipherSuite;
use sha2::Sha256;
pub use std::{borrow::ToOwned, string::String};

use zeroize::Zeroize;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, Zeroize)]
/// An example instance of ICE-FROST over Secp256k1 with SHA-256 as underlying hasher.
pub struct Ed25519Sha256;

impl CipherSuite for Ed25519Sha256 {
    type G = ark_ed25519::EdwardsProjective;

    type HashOutput = [u8; 32];

    type InnerHasher = Sha256;

    type Cipher = Aes128Gcm;

    fn context_string() -> String {
        "ICE-FROST_ED25519_SHA256".to_owned()
    }
}
