use gadget_keystore::backends::tangle::TangleBackend;
use gadget_keystore::backends::{Backend, BackendConfig};
use gadget_keystore::key_types::k256_ecdsa::K256Ecdsa;
use gadget_keystore::storage::FileStorage;
use gadget_keystore::Keystore;
use gadget_keystore::Result;

#[test]
fn fs_keystore() -> Result<()> {
    const EXPECTED: &[u8] = b"0242c73919c4fe51b8d4a876432d85c4d7b34e27685d530f92bc9f23849e6026b1";

    let tmp_dir = tempfile::tempdir()?;
    let mut keystore = Keystore::new();

    let file_storage = FileStorage::new(tmp_dir)?;
    keystore.register_storage::<K256Ecdsa>(BackendConfig::Local(Box::new(file_storage) as _), 0)?;

    keystore.ecdsa_generate_from_string("foo")?;
    let mut iter = keystore.iter_ecdsa();

    let key = iter.next().unwrap();
    assert_eq!(key.0, &*hex::decode(EXPECTED).unwrap());

    assert!(iter.next().is_none());

    Ok(())
}
