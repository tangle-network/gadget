use color_eyre;
use sp_core::{ecdsa, sr25519};

pub trait SubstrateKeystore {
    fn ecdsa_key(&self) -> color_eyre::Result<ecdsa::Pair>;

    fn sr25519_key(&self) -> color_eyre::Result<sr25519::Pair>;
}