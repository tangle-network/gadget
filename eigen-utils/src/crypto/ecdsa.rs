use aes::cipher::{KeyIvInit, StreamCipher};
use aes::{cipher, Aes128};
use alloy_primitives::Address;
use ctr::Ctr128BE;
use k256::ecdsa::{SigningKey, VerifyingKey};
use k256::{FieldBytes, PublicKey, SecretKey};
use rand::Rng;
use scrypt::{scrypt, Params};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, BufWriter, ErrorKind, Write};
use std::path::Path;
use std::str::FromStr;
use uuid::Uuid;

type Aes128Ctr = Ctr128BE<Aes128>;

#[derive(Debug, Serialize, Deserialize)]
struct CipherParamsJSON {
    #[serde(rename = "IV")]
    iv: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CryptoJSON {
    #[serde(rename = "Cipher")]
    cipher: String,
    #[serde(rename = "CipherText")]
    cipher_text: String,
    #[serde(rename = "CipherParams")]
    cipher_params: CipherParamsJSON,
    #[serde(rename = "KDF")]
    kdf: String,
    #[serde(rename = "KDFParams")]
    kdf_params: serde_json::Value,
    #[serde(rename = "MAC")]
    mac: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct EncryptedKeyJSONV3 {
    address: String,
    crypto: CryptoJSON,
    id: String,
    version: String,
}

#[derive(Debug)]
struct Key {
    id: Uuid,
    address: VerifyingKey,
    private_key: SecretKey,
}

#[derive(Serialize, Deserialize)]
pub struct Keystore {
    id: Uuid,
    address: String,
    private_key: Vec<u8>,
    // nonce: [u8; 12],
}

impl Key {
    fn new(secret_key: SecretKey) -> Self {
        let address = VerifyingKey::from(secret_key.public_key());

        Self {
            id: Uuid::new_v4(),
            address,
            private_key: secret_key,
        }
    }
}

fn public_key_to_address(public_key: &[u8; 65]) -> String {
    let mut hasher = Keccak256::new();
    hasher.update(&public_key[1..]);
    let result = hasher.finalize();
    hex::encode(&result[12..])
}

fn aes_ctr_xor(key: &[u8], in_text: &[u8], iv: &[u8]) -> Result<Vec<u8>, cipher::InvalidLength> {
    // AES-128 is selected due to size of encrypt_key.
    let mut cipher = Aes128Ctr::new(key.into(), iv.into());

    let mut out_text = in_text.to_vec();
    cipher.apply_keystream(&mut out_text);

    Ok(out_text)
}

fn encrypt_data_v3(
    data: &[u8],
    auth: &[u8],
    scrypt_n: u8,
    scrypt_p: u32,
) -> Result<CryptoJSON, Box<dyn Error>> {
    let mut salt = [0u8; 32];
    rand::thread_rng().fill(&mut salt);
    let scrypt_params = Params::new(scrypt_n as u8, 8, scrypt_p, 32)?;
    let mut derived_key = [0u8; 32];
    scrypt(auth, &salt, &scrypt_params, &mut derived_key)?;

    let encrypt_key = &derived_key[..16];
    let mut iv = [0u8; 16];
    rand::thread_rng().fill(&mut iv);
    let cipher_text = aes_ctr_xor(encrypt_key, data, &iv)?;

    let mut hasher = Keccak256::new();
    hasher.update(&derived_key[16..32]);
    hasher.update(&cipher_text);
    let mac = hasher.finalize();

    let scrypt_params_json = serde_json::json!({
        "n": scrypt_n,
        "r": 8,
        "p": scrypt_p,
        "dklen": 32,
        "salt": hex::encode(salt),
    });

    let cipher_params_json = CipherParamsJSON {
        iv: hex::encode(iv),
    };

    let crypto_struct = CryptoJSON {
        cipher: String::from("aes-128-ctr"),
        cipher_text: hex::encode(cipher_text),
        cipher_params: cipher_params_json,
        kdf: String::from("scrypt"),
        kdf_params: scrypt_params_json,
        mac: hex::encode(mac),
    };

    Ok(crypto_struct)
}

fn encrypt_key(
    keystore: &Keystore,
    auth: &str,
    scrypt_n: u8,
    scrypt_p: u32,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let crypto_struct =
        encrypt_data_v3(&keystore.private_key, auth.as_bytes(), scrypt_n, scrypt_p)?;
    let encrypted_key_json_v3 = EncryptedKeyJSONV3 {
        address: keystore.address.clone().to_string(),
        crypto: crypto_struct,
        id: keystore.id.to_string(),
        version: String::from("3"),
    };

    Ok(serde_json::to_vec(&encrypted_key_json_v3)?)
}

fn decrypt_data_v3(crypto_json: &CryptoJSON, auth: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    if crypto_json.cipher != "aes-128-ctr" {
        return Err(Box::new(io::Error::new(
            io::ErrorKind::InvalidInput,
            "cipher not supported",
        )));
    }

    let mac = hex::decode(&crypto_json.mac)?;
    let iv = hex::decode(&crypto_json.cipher_params.iv)?;
    let cipher_text = hex::decode(&crypto_json.cipher_text)?;

    let kdf_params: serde_json::Value = serde_json::from_value(crypto_json.kdf_params.clone())?;
    let salt = hex::decode(kdf_params["salt"].as_str().ok_or("missing salt")?)?;
    let scrypt_n = kdf_params["n"].as_u64().ok_or("missing n")? as u8;
    let scrypt_p = kdf_params["p"].as_u64().ok_or("missing p")? as u32;
    let scrypt_params = Params::new(
        scrypt_n,
        Params::RECOMMENDED_R,
        scrypt_p,
        Params::RECOMMENDED_LEN,
    )?;

    let mut derived_key = [0u8; 32];
    scrypt(auth.as_bytes(), &salt, &scrypt_params, &mut derived_key)?;

    let mut hasher = Keccak256::new();
    hasher.update(&derived_key[16..32]);
    hasher.update(&cipher_text);
    let calculated_mac = hasher.finalize();

    if mac != calculated_mac.as_slice() {
        return Err(Box::new(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid MAC",
        )));
    }

    let plain_text = aes_ctr_xor(&derived_key[..16], &cipher_text, &iv)?;
    Ok(plain_text)
}

fn decrypt_key(key_json: &[u8], auth: &str) -> Result<Key, Box<dyn Error>> {
    let m: serde_json::Value = serde_json::from_slice(key_json)?;

    let version = m["version"].as_str().ok_or("missing version")?;
    let (key_bytes, key_id) = if version == "3" {
        let k: EncryptedKeyJSONV3 = serde_json::from_slice(key_json)?;
        let key_bytes = decrypt_data_v3(&k.crypto, auth)?;
        let key_id = Uuid::parse_str(&k.id)?;
        (key_bytes, key_id)
    } else {
        return Err(Box::new(io::Error::new(
            io::ErrorKind::InvalidInput,
            "unsupported version",
        )));
    };

    let private_key = {
        let mut key_array = [0u8; 32];
        key_array.copy_from_slice(&key_bytes);
        key_array
    };

    let signing_key = SigningKey::try_from(private_key.as_slice())?;
    let secret_key = SecretKey::from(signing_key.clone());
    let verifying_key = VerifyingKey::from(&signing_key);

    let key = Key {
        id: key_id,
        address: verifying_key,
        private_key: secret_key,
    };

    Ok(key)
}

pub fn write_key_from_hex(path: &str, private_key_hex: &str, password: &str) -> io::Result<()> {
    let private_key_bytes = hex::decode(private_key_hex)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
    let secret_key = SecretKey::from_bytes(FieldBytes::from_slice(&private_key_bytes))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid private key"))?;
    write_key(path, &secret_key, password)
}

pub fn write_key(path: &str, private_key: &SecretKey, password: &str) -> io::Result<()> {
    let id = Uuid::new_v4();
    let public_key = VerifyingKey::from(private_key.public_key());
    // let mut nonce = [0u8; 12];
    // OsRng.fill_bytes(&mut nonce);

    let key = Keystore {
        id,
        address: public_key.to_address().to_string(),
        private_key: private_key.to_bytes().to_vec(),
        // nonce,
    };

    let encrypted_bytes = encrypt_key(
        &key,
        password,
        Params::RECOMMENDED_LOG_N,
        Params::RECOMMENDED_P,
    )
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
    write_bytes_to_file(path, &encrypted_bytes)
}

pub fn write_bytes_to_file(path: &str, data: &[u8]) -> io::Result<()> {
    let path = Path::new(path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(data)?;
    writer.flush()?;
    Ok(())
}

fn password_to_key(password: &str) -> [u8; 32] {
    use sha3::Digest;
    let mut hasher = sha3::Sha3_256::new();
    hasher.update(password.as_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result[..32]);
    key
}

pub fn read_key(key_store_file: &str, password: &str) -> io::Result<SecretKey> {
    let key_store_contents = fs::read(key_store_file)?;
    let key: Key = decrypt_key(&key_store_contents, password)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
    // SecretKey::from_bytes(FieldBytes::from_slice(&key.private_key)).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid private key"))
    Ok(key.private_key)
}

pub fn get_address_from_keystore_file(key_store_file: &str) -> io::Result<Address> {
    let key_json = fs::read(key_store_file)?;
    let keystore: Keystore = serde_json::from_slice(&key_json)?;
    Ok(Address::from_str(&keystore.address).map_err(|e| io::Error::new(ErrorKind::Other, e))?)
}

pub trait ToAddress {
    fn to_address(&self) -> Address;
}

impl ToAddress for VerifyingKey {
    fn to_address(&self) -> Address {
        use sha3::{Digest, Keccak256};
        let public_key = self.to_encoded_point(false);
        let public_key_bytes = public_key.as_bytes();
        let hash = Keccak256::digest(&public_key_bytes[1..]);
        Address::from_slice(&hash[12..])
    }
}
