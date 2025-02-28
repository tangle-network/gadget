use crate::error::Error;
use gadget_keystore::backends::bn254::Bn254Backend;
use gadget_keystore::backends::eigenlayer::EigenlayerBackend;
use gadget_keystore::{Keystore, KeystoreConfig};
use gadget_std::path::Path;

pub const ANVIL_PRIVATE_KEYS: [&str; 10] = [
    "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
    "59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d",
    "5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a",
    "7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6",
    "47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733639f19c30a34926a",
    "8b3a350cf5c34c9194ca85829a2df0ec3153be0318b5e2d3348e872092edffba",
    "92db14e403b83dfe3df233f83dfa3a0d7096f21ca9b0d6d6b8d88b2b4ec1564e",
    "4bbbf85ce3377467afe5d46f804f221813b2bb87f24d81f60f1fcdbf7cbf4356",
    "dbda1821b80551c9d65939329250298aa3472ba22feea921c0cf5d620ea67b97",
    "2a871d0798f97d79848a013d4936a73bf4cc922c825d33c1cf7073dff6d409c6",
];

/// Injects a key for use with an Anvil Testnet using the given seed string
///
/// # Keys Generated
/// - `ECDSA`: Anvil Dev Key
/// - `BLS BN254`: Random
///
/// # Errors
/// - Fails if the given index is out of bounds
/// - May fail if the keystore path cannot be created or accessed
pub fn inject_anvil_key<P: AsRef<Path>>(keystore_path: P, seed: &str) -> Result<(), Error> {
    let keystore_path = keystore_path.as_ref();
    if !keystore_path.exists() {
        std::fs::create_dir_all(keystore_path).map_err(|e| Error::Keystore(e.into()))?;
    }
    let config = KeystoreConfig::new().fs_root(keystore_path);
    let keystore = Keystore::new(config)?;

    keystore.ecdsa_generate_from_string(seed)?;
    keystore.bls_bn254_generate_new(None)?;

    Ok(())
}
