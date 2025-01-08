// use gadget_keystore::Error as KeystoreError;
// use std::path::Path;

// /// Injects the pre-made Tangle keys of the given index where that index is 0-4
// ///
// /// # Keys Generated
// /// - `SR25519`: Tangle Dev Key
// /// - `ED25519`: Tangle Dev Key
// /// - `ECDSA`: Tangle Dev Key
// /// - `BLS BN254`: Random
// /// - `BLS381`: Random
// ///
// /// # Indices
// /// - 0: Alice
// /// - 1: Bob
// /// - 2: Charlie
// /// - 3: Dave
// /// - 4: Eve
// ///
// /// # Errors
// /// - Fails if the given index is out of bounds
// /// - May fail if the keystore path cannot be created or accessed
// fn inject_tangle_key<P: AsRef<Path>>(keystore_path: P, name: &str) -> Result<(), KeystoreError> {
// TODO: finish updating to match our new keystore
// let keystore = GenericKeyStore::<parking_lot::RawRwLock>::Fs(FilesystemKeystore::open(
//     keystore_path.as_ref(),
// )?);
//
// let suri = format!("//{name}");
//
// let sr = sp_core::sr25519::Pair::from_string(&suri, None)
//     .map_err(|_| KeystoreError::InvalidSeed("Invalid SR25519 keypair".into()))?;
//
// let sr_seed = &sr.as_ref().secret.to_bytes();
//
// let ed = sp_core::ed25519::Pair::from_string(&suri, None)
//     .map_err(|_| KeystoreError::InvalidSeed("Invalid ED25519 keypair".into()))?;
// let ed_seed = &ed.seed();
//
// let ecdsa = sp_core::ecdsa::Pair::from_string(&suri, None)
//     .map_err(|_| KeystoreError::InvalidSeed("Invalid ECDSA keypair".into()))?;
// let ecdsa_seed = ecdsa.seed();
//
// keystore.sr25519_generate_new(Some(sr_seed))?;
// keystore.ed25519_generate_new(Some(ed_seed))?;
// keystore.ecdsa_generate_new(Some(&ecdsa_seed))?;
// keystore.bls381_generate_new(None)?;
// keystore.bls_bn254_generate_new(None)?;
//
// // Perform sanity checks on conversions between secrets to ensure
// // consistency as the program executes
// let bytes: [u8; 64] = sr.as_ref().secret.to_bytes();
// let secret_key_again = gadget_keystore::sr25519::secret_from_bytes(&bytes)
//     .map_err(|_| KeystoreError::InvalidSeed("Invalid SR25519 Bytes".into()))?;
// assert_eq!(&bytes[..], &secret_key_again.to_bytes()[..]);
//
// let sr2 = TanglePairSigner::new(
//     sp_core::sr25519::Pair::from_seed_slice(&bytes)
//         .map_err(|_| KeystoreError::InvalidSeed("Invalid SR25519 keypair".into()))?,
// );
//
// let sr1_account_id: AccountId32 = AccountId32(sr.as_ref().public.to_bytes());
// let sr2_account_id: AccountId32 = sr2.account_id().clone();
// assert_eq!(sr1_account_id, sr2_account_id);
//
// match keystore.ecdsa_key() {
//     Ok(ecdsa_key) => {
//         assert_eq!(ecdsa_key.signer().seed(), ecdsa_seed);
//     }
//     Err(err) => {
//         gadget_logging::error!(target: "gadget", "Failed to load ecdsa key: {err}");
//         return Err(KeystoreError::KeyNotFound);
//     }
// }
//
// match keystore.sr25519_key() {
//     Ok(sr25519_key) => {
//         assert_eq!(sr25519_key.signer().public().0, sr.public().0);
//     }
//     Err(err) => {
//         gadget_logging::error!(target: "gadget", "Failed to load sr25519 key: {err}");
//         return Err(KeystoreError::KeyNotFound);
//     }
// }
//
// match keystore.ed25519_key() {
//     Ok(ed25519_key) => {
//         assert_eq!(ed25519_key.signer().public().0, ed.public().0);
//     }
//     Err(err) => {
//         gadget_logging::error!(target: "gadget", "Failed to load ed25519 key: {err}");
//         return Err(KeystoreError::KeyNotFound);
//     }
// }

//     Ok(())
// }
