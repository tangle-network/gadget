use crate::io::error::Result;
use sp_core::{ecdsa, sr25519};

pub trait SubstrateKeystore {
    fn ecdsa_key(&self) -> Result<ecdsa::Pair>;

    fn sr25519_key(&self) -> Result<sr25519::Pair>;
}
