pub use gadget_io::SubstrateKeystore;
use sp_core::{ecdsa, sr25519};

pub fn load_keys_from_keystore<T: SubstrateKeystore>(
    keystore_config: &T,
) -> color_eyre::Result<(ecdsa::Pair, sr25519::Pair)> {
    Ok((keystore_config.ecdsa_key()?, keystore_config.sr25519_key()?))
}
