use gadget_keystore::backends::bn254::Bn254Backend;
use gadget_keystore::backends::tangle::bls::TangleBlsBackend;
use gadget_keystore::backends::tangle::TangleBackend;
use gadget_keystore::{Keystore, KeystoreConfig};
use std::path::Path;

/// Injects the pre-made Tangle keys of the given name
///
/// # Keys Generated
/// - `SR25519`: Tangle Dev Key
/// - `ED25519`: Tangle Dev Key
/// - `ECDSA`: Tangle Dev Key
/// - `BLS381`: Tangle Dev Key
/// - `BLS377`: Tangle Dev Key
/// - `BLS BN254`: Random
///
/// # Names
/// - "alice"
/// - "bob"
/// - "charlie"
/// - "dave"
/// - "eve"
///
/// # Errors
/// - May fail if the keystore path cannot be created or accessed
/// - May fail if the key generation fails
pub fn inject_tangle_key<P: AsRef<Path>>(
    keystore_path: P,
    name: &str,
) -> Result<(), gadget_keystore::Error> {
    let config = KeystoreConfig::new().fs_root(keystore_path.as_ref());
    let keystore = Keystore::new(config)?;

    keystore.sr25519_generate_from_string(name)?;
    keystore.ed25519_generate_from_string(name)?;
    keystore.ecdsa_generate_from_string(name)?;
    keystore.bls381_generate_from_string(name)?;
    keystore.bls377_generate_from_string(name)?;
    keystore.bls_bn254_generate_new(None)?;

    Ok(())
}
