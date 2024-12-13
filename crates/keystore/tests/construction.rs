use gadget_keystore::backends::tangle::TangleBackend;
use gadget_keystore::Result;
use gadget_keystore::{Keystore, KeystoreConfig};

#[test]
fn fs_keystore() -> Result<()> {
    const EXPECTED: &[u8] = b"0242c73919c4fe51b8d4a876432d85c4d7b34e27685d530f92bc9f23849e6026b1";

    let tmp_dir = tempfile::tempdir()?;
    let keystore = Keystore::new(KeystoreConfig::new().fs_root(tmp_dir.path().to_path_buf()))?;

    keystore.ecdsa_generate_from_string("foo")?;
    let mut iter = keystore.iter_ecdsa();

    let key = iter.next().unwrap();
    assert_eq!(key.0, &*hex::decode(EXPECTED).unwrap());

    assert!(iter.next().is_none());

    Ok(())
}
