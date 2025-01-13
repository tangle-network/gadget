use gadget_crypto::sp_core::SpEcdsa;
use gadget_keystore::backends::tangle::TangleBackend;
use gadget_keystore::backends::Backend;
use gadget_keystore::Result;
use gadget_keystore::{Keystore, KeystoreConfig};

#[test]
fn fs_keystore() -> Result<()> {
    const EXPECTED: &[u8] = b"03065d2080364c71dccbdc7e3f552dc3a4501e02751211d06d4898e8e0e0509e30";

    let tmp_dir = tempfile::tempdir()?;
    let keystore = Keystore::new(KeystoreConfig::new().fs_root(tmp_dir.path()))?;

    keystore.ecdsa_generate_from_string("//Foo")?;

    let ecdsa_keys = keystore.list_local::<SpEcdsa>()?;
    assert_eq!(ecdsa_keys.len(), 1);
    assert_eq!(ecdsa_keys[0].0 .0, &*hex::decode(EXPECTED).unwrap());

    Ok(())
}
