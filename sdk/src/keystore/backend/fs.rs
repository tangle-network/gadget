//! Filesystem-based keystore backend.

use crate::keystore::bn254::Public;
use crate::keystore::{
    bls381, bn254, ecdsa, ed25519, sr25519, Backend, Error, KeystoreUriSanitizer,
};
use crate::{debug, warn};
use alloc::string::ToString;
use ark_serialize::CanonicalSerialize;
use core::str::FromStr;
use std::{fs, io::Write, path::PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
enum KeyType {
    Sr25519 = 0x00,
    Ed25519 = 0x01,
    Ecdsa = 0x02,
    Bls381 = 0x03,
    BlsBn254 = 0x04,
}

/// The filesystem keystore backend.
///
/// This stores keys in files, where each file is named after the public key and contains the
/// private key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesystemKeystore {
    /// The root directory where the keys are stored.
    root: PathBuf,
}

impl FilesystemKeystore {
    /// Open the store at the given path.
    ///
    /// # Errors
    ///
    /// An error will be returned if the root directory cannot be created.
    pub fn open<T: KeystoreUriSanitizer>(path: T) -> Result<Self, Error> {
        let root = path.sanitize_file_path();
        fs::create_dir_all(&root)?;

        Ok(Self { root })
    }

    /// Write the given `data` to `file`.
    fn write_to_file(file: PathBuf, data: &[u8]) -> Result<(), Error> {
        let mut file = fs::File::create(file).unwrap();

        #[cfg(target_family = "unix")]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(fs::Permissions::from_mode(0o600))?;
        }

        file.write_all(data)?;
        file.flush()?;
        Ok(())
    }

    /// Get the file path for the given public key and key type.
    fn key_file_path(&self, public: &[u8], key_type: KeyType) -> PathBuf {
        let key_type = key_type as u16;
        let public_key = hex::encode(public);
        let key_type = hex::encode(key_type.to_be_bytes());
        let file_name = format!("{key_type}{public_key}");
        self.root.join(file_name)
    }

    /// Get the key phrase for a given public key and key type.
    fn secret_by_type(&self, public: &[u8], key_type: KeyType) -> Result<Option<Vec<u8>>, Error> {
        let path = self.key_file_path(public, key_type);
        if path.exists() {
            let content = fs::read(&path)?;
            if content.is_empty() {
                return Ok(None);
            }
            // check if the contents are hex encoded
            // if so, we need to decode them
            if content.iter().all(|&b| b.is_ascii_hexdigit()) {
                if let Ok(decoded) = hex::decode(&content) {
                    debug!("Decoded hex-encoded key from file {:?}", path);
                    Ok(Some(decoded))
                } else {
                    warn!("Invalid hex encoding in file {:?}", path);
                    Ok(None)
                }
            } else {
                Ok(Some(content))
            }
        } else {
            Ok(None)
        }
    }

    /// Iterate over all public keys of a given key type.
    /// skipping any invalid files.
    fn iter_keys(&self, key_type: KeyType) -> Box<dyn Iterator<Item = Vec<u8>>> {
        let key_type_u16 = key_type as u16;
        let key_type_bytes = key_type_u16.to_be_bytes();
        let prefix = hex::encode(key_type_bytes);
        let res = fs::read_dir(&self.root);
        match res {
            Ok(r) => Box::new(r.filter_map(move |entry| {
                let entry = entry.ok()?;
                let file_name = entry.file_name().into_string().ok()?;

                // If the key type is BlsBn254, search for .pub files. Otherwise, search normally
                if file_name.starts_with(&prefix) {
                    if key_type == KeyType::BlsBn254 {
                        if file_name.ends_with(".pub") {
                            let public_key_path = entry.path();
                            let public_key_bytes = fs::read(public_key_path).ok()?;
                            Some(public_key_bytes)
                        } else {
                            None
                        }
                    } else {
                        let public_key_file = file_name.strip_prefix(&prefix)?;
                        hex::decode(public_key_file).ok()
                    }
                } else {
                    None
                }
            })),
            Err(_) => Box::new(core::iter::empty()),
        }
    }
}

impl Backend for FilesystemKeystore {
    fn sr25519_generate_new(&self, seed: Option<&[u8]>) -> Result<sr25519::Public, Error> {
        let secret = sr25519::generate_with_optional_seed(seed)?;
        let public = secret.to_public();
        let path = self.key_file_path(public.as_ref(), KeyType::Sr25519);
        Self::write_to_file(path, &secret.to_bytes())?;
        Ok(public)
    }

    fn sr25519_sign(
        &self,
        public: &sr25519::Public,
        msg: &[u8],
    ) -> Result<Option<sr25519::Signature>, Error> {
        let secret_bytes = self.secret_by_type(public.as_ref(), KeyType::Sr25519)?;
        if let Some(buf) = secret_bytes {
            let secret = sr25519::secret_from_bytes(&buf)?;
            Ok(Some(sr25519::sign(&secret, msg)?))
        } else {
            Ok(None)
        }
    }

    fn ed25519_generate_new(&self, seed: Option<&[u8]>) -> Result<ed25519::Public, Error> {
        let secret = ed25519::generate_with_optional_seed(seed)?;
        let public = ed25519::to_public(&secret);
        let path = self.key_file_path(public.as_ref(), KeyType::Ed25519);
        Self::write_to_file(path, secret.as_ref())?;
        Ok(public)
    }

    fn ed25519_sign(
        &self,
        public: &ed25519::Public,
        msg: &[u8],
    ) -> Result<Option<ed25519::Signature>, Error> {
        let secret_bytes = self.secret_by_type(public.as_ref(), KeyType::Ed25519)?;
        if let Some(buf) = secret_bytes {
            let secret = ed25519::secret_from_bytes(&buf)?;
            Ok(Some(ed25519::sign(&secret, msg)))
        } else {
            Ok(None)
        }
    }

    fn ecdsa_generate_new(&self, seed: Option<&[u8]>) -> Result<ecdsa::Public, Error> {
        let secret = ecdsa::generate_with_optional_seed(seed)
            .map_err(|err| Error::Ecdsa(err.to_string()))?;
        let public = secret.public_key();
        let path = self.key_file_path(&public.to_sec1_bytes(), KeyType::Ecdsa);
        Self::write_to_file(path, &secret.to_bytes()[..])?;
        Ok(public)
    }

    fn ecdsa_sign(
        &self,
        public: &ecdsa::Public,
        msg: &[u8],
    ) -> Result<Option<ecdsa::Signature>, Error> {
        let secret_bytes = self.secret_by_type(&public.to_sec1_bytes(), KeyType::Ecdsa)?;
        if let Some(buf) = secret_bytes {
            let secret =
                ecdsa::secret_from_bytes(&buf).map_err(|err| Error::Ecdsa(err.to_string()))?;
            Ok(Some(ecdsa::sign(&secret, msg)))
        } else {
            Ok(None)
        }
    }

    fn bls381_generate_new(&self, seed: Option<&[u8]>) -> Result<bls381::Public, Error> {
        use w3f_bls::SerializableToBytes;

        let secret = bls381::generate_with_optional_seed(seed);
        let public = bls381::to_public(&secret);
        let path = self.key_file_path(&public.to_bytes(), KeyType::Bls381);
        Self::write_to_file(path, &secret.to_bytes())?;
        Ok(public)
    }

    fn bls381_sign(
        &self,
        public: &bls381::Public,
        msg: &[u8],
    ) -> Result<Option<bls381::Signature>, Error> {
        use w3f_bls::SerializableToBytes;

        let secret_bytes = self.secret_by_type(&public.to_bytes(), KeyType::Bls381)?;
        if let Some(buf) = secret_bytes {
            let mut secret = bls381::secret_from_bytes(&buf);
            Ok(Some(bls381::sign(&mut secret, msg)))
        } else {
            Ok(None)
        }
    }

    fn bls_bn254_generate_new(&self, seed: Option<&[u8]>) -> Result<bn254::Public, Error> {
        let secret = bn254::generate_with_optional_seed(seed);
        let public = bn254::to_public(&secret);
        let path = self.key_file_path(
            bn254::hash_public(public.clone())?.as_bytes(),
            KeyType::BlsBn254,
        );

        // Serialize and store the secret key
        let mut secret_bytes = Vec::new();
        secret
            .serialize_uncompressed(&mut secret_bytes)
            .map_err(|e| Error::BlsBn254(e.to_string()))?;
        Self::write_to_file(path.clone(), &secret_bytes)?;

        // Store the public key in metadata file
        let public_key_path = path.with_extension("pub");
        Self::write_to_file(
            public_key_path,
            serde_json::to_vec(&public)
                .map_err(|e| Error::BlsBn254(e.to_string()))?
                .as_slice(),
        )?;

        Ok(public)
    }

    fn bls_bn254_generate_from_secret(&self, secret: String) -> Result<Public, Error> {
        let pair = eigensdk::crypto_bls::BlsKeyPair::new(secret.clone())
            .map_err(|e| Error::BlsBn254(e.to_string()))?;

        let public = pair.public_key();

        let path = self.key_file_path(
            bn254::hash_public(public.clone())?.as_bytes(),
            KeyType::BlsBn254,
        );
        let secret = bn254::Secret::from_str(&secret)
            .map_err(|_| Error::BlsBn254("Invalid BLS BN254 secret".to_string()))?;

        // Serialize and store the secret key
        let mut secret_bytes = Vec::new();
        secret
            .serialize_uncompressed(&mut secret_bytes)
            .map_err(|e| Error::BlsBn254(e.to_string()))?;
        Self::write_to_file(path.clone(), &secret_bytes)?;

        // Store the public key in metadata file
        let public_key_path = path.with_extension("pub");
        Self::write_to_file(
            public_key_path,
            serde_json::to_vec(&public)
                .map_err(|e| Error::BlsBn254(e.to_string()))?
                .as_slice(),
        )?;

        Ok(public)
    }

    fn bls_bn254_sign(
        &self,
        public: &bn254::Public,
        msg: &[u8; 32],
    ) -> Result<Option<bn254::Signature>, Error> {
        let hashed_public = bn254::hash_public(public.clone())?;
        let secret_bytes = self.secret_by_type(hashed_public.as_bytes(), KeyType::BlsBn254)?;
        if let Some(buf) = secret_bytes {
            let mut secret = bn254::secret_from_bytes(&buf)?;
            Ok(Some(bn254::sign(&mut secret, msg)))
        } else {
            Ok(None)
        }
    }

    fn expose_sr25519_secret(
        &self,
        public: &sr25519::Public,
    ) -> Result<Option<sr25519::Secret>, Error> {
        let secret_bytes = self.secret_by_type(public.as_ref(), KeyType::Sr25519)?;
        if let Some(buf) = secret_bytes {
            let secret = sr25519::secret_from_bytes(&buf)?;
            Ok(Some(secret))
        } else {
            Ok(None)
        }
    }

    fn expose_ecdsa_secret(&self, public: &ecdsa::Public) -> Result<Option<ecdsa::Secret>, Error> {
        let secret_bytes = self.secret_by_type(&public.to_sec1_bytes(), KeyType::Ecdsa)?;
        if let Some(buf) = secret_bytes {
            Ok(Some(
                ecdsa::secret_from_bytes(&buf).map_err(|err| Error::Ecdsa(err.to_string()))?,
            ))
        } else {
            Ok(None)
        }
    }

    fn expose_ed25519_secret(
        &self,
        public: &ed25519::Public,
    ) -> Result<Option<ed25519::Secret>, Error> {
        let secret_bytes = self.secret_by_type(public.as_ref(), KeyType::Ed25519)?;
        if let Some(buf) = secret_bytes {
            Ok(Some(ed25519::secret_from_bytes(&buf)?))
        } else {
            Ok(None)
        }
    }

    fn expose_bls381_secret(
        &self,
        public: &bls381::Public,
    ) -> Result<Option<bls381::Secret>, Error> {
        use w3f_bls::SerializableToBytes;

        let secret_bytes = self.secret_by_type(&public.to_bytes(), KeyType::Bls381)?;
        if let Some(buf) = secret_bytes {
            Ok(Some(bls381::secret_from_bytes(&buf)))
        } else {
            Ok(None)
        }
    }

    fn expose_bls_bn254_secret(
        &self,
        public: &bn254::Public,
    ) -> Result<Option<bn254::Secret>, Error> {
        let hashed_public = bn254::hash_public(public.clone())?;
        let secret_bytes = self.secret_by_type(hashed_public.as_bytes(), KeyType::BlsBn254)?;
        if let Some(buf) = secret_bytes {
            Ok(Some(bn254::secret_from_bytes(&buf)?))
        } else {
            Ok(None)
        }
    }

    fn iter_sr25519(&self) -> impl Iterator<Item = sr25519::Public> {
        self.iter_keys(KeyType::Sr25519)
            .flat_map(|b| sr25519::Public::from_bytes(&b))
    }

    fn iter_ecdsa(&self) -> impl Iterator<Item = ecdsa::Public> {
        self.iter_keys(KeyType::Ecdsa)
            .flat_map(|b| ecdsa::Public::from_sec1_bytes(&b))
    }

    fn iter_ed25519(&self) -> impl Iterator<Item = ed25519::Public> {
        self.iter_keys(KeyType::Ed25519)
            .flat_map(|b| ed25519::Public::try_from(b.as_slice()))
    }

    fn iter_bls381(&self) -> impl Iterator<Item = bls381::Public> {
        use w3f_bls::SerializableToBytes;

        self.iter_keys(KeyType::Bls381)
            .flat_map(|b| bls381::Public::from_bytes(&b))
    }

    fn iter_bls_bn254(&self) -> impl Iterator<Item = bn254::Public> {
        self.iter_keys(KeyType::BlsBn254)
            .flat_map(|b| serde_json::from_slice(&b).ok())
    }
}
