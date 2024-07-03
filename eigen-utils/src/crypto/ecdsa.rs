use std::fs::{self, File};
use std::io::{self, Write, BufWriter, ErrorKind};
use std::path::Path;

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};

use alloy_primitives::Address;
use k256::ecdsa::{SigningKey, VerifyingKey};
use k256::{FieldBytes, SecretKey};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use aes::Aes128;
use aes::cipher::{StreamCipher, StreamCipherSeek, KeyIvInit};
use aes::cipher::generic_array::GenericArray;
use rand::Rng;
use scrypt::{scrypt, Params};
use sha3::{Digest, Keccak256};
use std::error::Error;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize)]
struct CipherParamsJSON {
    IV: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CryptoJSON {
    Cipher: String,
    CipherText: String,
    CipherParams: CipherParamsJSON,
    KDF: String,
    KDFParams: serde_json::Value,
    MAC: String,
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
    address: String,
    private_key: [u8; 32],
}

#[derive(Serialize, Deserialize)]
pub struct Keystore {
    id: Uuid,
    address: String,
    private_key: Vec<u8>,
    // nonce: [u8; 12],
}

impl Key {
    fn new(private_key: [u8; 32]) -> Self {
        let public_key = public_key_from_private_key(&private_key);
        let address = public_key_to_address(&public_key);

        Self {
            id: Uuid::new_v4(),
            address,
            private_key,
        }
    }
}

fn public_key_from_private_key(private_key: &[u8; 32]) -> [u8; 65] {
    // Implementation to derive the public key from the private key
    unimplemented!()
}

fn public_key_to_address(public_key: &[u8; 65]) -> String {
    let mut hasher = Keccak256::new();
    hasher.update(&public_key[1..]);
    let result = hasher.finalize();
    hex::encode(&result[12..])
}

fn aes_ctr_xor(key: &[u8], data: &[u8], iv: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let key = GenericArray::from_slice(key);
    let iv = GenericArray::from_slice(iv);
    let mut cipher = Aes128::new(&key);
    let mut buffer = data.to_vec();
    StreamCipher::apply_keystream(&mut cipher, &mut buffer);
    Ok(buffer)
}

fn encrypt_data_v3(data: &[u8], auth: &[u8], scrypt_n: u8, scrypt_p: u32) -> Result<CryptoJSON, Box<dyn Error>> {
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
        IV: hex::encode(iv),
    };

    let crypto_struct = CryptoJSON {
        Cipher: String::from("aes-128-ctr"),
        CipherText: hex::encode(cipher_text),
        CipherParams: cipher_params_json,
        KDF: String::from("scrypt"),
        KDFParams: scrypt_params_json,
        MAC: hex::encode(mac),
    };

    Ok(crypto_struct)
}

fn encrypt_key(keystore: &Keystore, auth: &str, scrypt_n: u8, scrypt_p: u32) -> Result<Vec<u8>, Box<dyn Error>> {
    let crypto_struct = encrypt_data_v3(&keystore.private_key, auth.as_bytes(), scrypt_n, scrypt_p)?;
    let encrypted_key_json_v3 = EncryptedKeyJSONV3 {
        address: keystore.address.clone().to_string(),
        crypto: crypto_struct,
        id: keystore.id.to_string(),
        version: String::from("3"),
    };

    Ok(serde_json::to_vec(&encrypted_key_json_v3)?)
}

fn decrypt_data_v3(crypto_json: &CryptoJSON, auth: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    if crypto_json.Cipher != "aes-128-ctr" {
        return Err(Box::new(io::Error::new(io::ErrorKind::InvalidInput, "cipher not supported")));
    }

    let mac = hex::decode(&crypto_json.MAC)?;
    let iv = hex::decode(&crypto_json.CipherParams.IV)?;
    let cipher_text = hex::decode(&crypto_json.CipherText)?;

    let kdf_params: serde_json::Value = serde_json::from_value(crypto_json.KDFParams.clone())?;
    let salt = hex::decode(kdf_params["salt"].as_str().ok_or("missing salt")?)?;
    let scrypt_n = kdf_params["n"].as_u64().ok_or("missing n")? as u8;
    let scrypt_p = kdf_params["p"].as_u64().ok_or("missing p")? as u32;
    let scrypt_params = Params::new(scrypt_n, Params::RECOMMENDED_R, scrypt_p, Params::RECOMMENDED_LEN)?;

    let mut derived_key = [0u8; 32];
    scrypt(auth.as_bytes(), &salt, &scrypt_params, &mut derived_key)?;

    let mut hasher = Keccak256::new();
    hasher.update(&derived_key[16..32]);
    hasher.update(&cipher_text);
    let calculated_mac = hasher.finalize();

    if mac != calculated_mac.as_slice() {
        return Err(Box::new(io::Error::new(io::ErrorKind::InvalidInput, "invalid MAC")));
    }

    let plain_text = aes_ctr_xor(&derived_key[..16], &cipher_text, &iv)?;
    Ok(plain_text)
}

fn decrypt_key(key_json: &[u8], auth: &str) -> Result<Keystore, Box<dyn Error>> {
    let m: serde_json::Value = serde_json::from_slice(key_json)?;

    let version = m["version"].as_str().ok_or("missing version")?;
    let (key_bytes, key_id) = if version == "3" {
        let k: EncryptedKeyJSONV3 = serde_json::from_slice(key_json)?;
        let key_bytes = decrypt_data_v3(&k.crypto, auth)?;
        let key_id = Uuid::parse_str(&k.id)?;
        (key_bytes, key_id)
    } else {
        return Err(Box::new(io::Error::new(io::ErrorKind::InvalidInput, "unsupported version")));
    };

    let private_key = {
        let mut key_array = [0u8; 32];
        key_array.copy_from_slice(&key_bytes);
        key_array
    };

    // let key = Key {
    //     id: key_id,
    //     address: public_key_to_address(&public_key_from_private_key(&private_key)),
    //     private_key,
    // };
    let keystore = Keystore {
        id: key_id,
        address: public_key_to_address(&public_key_from_private_key(&private_key)),
        private_key: private_key.to_vec(),
    };

    Ok(keystore)
}

pub fn write_key_from_hex(path: &str, private_key_hex: &str, password: &str) -> io::Result<()> {
    let private_key_bytes = hex::decode(private_key_hex)?;
    let secret_key = SecretKey::from_bytes(FieldBytes::from_slice(&private_key_bytes)).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid private key"))?;
    write_key(path, &secret_key, password)
}

pub fn write_key(path: &str, private_key: &SecretKey, password: &str) -> io::Result<()> {
    let id = Uuid::new_v4();
    let public_key = VerifyingKey::from(private_key);
    // let mut nonce = [0u8; 12];
    // OsRng.fill_bytes(&mut nonce);

    let key = Keystore {
        id,
        address: public_key.to_address().to_string(),
        private_key: private_key.to_bytes().to_vec(),
        // nonce,
    };

    let encrypted_bytes = encrypt_key(&key, password, Params::RECOMMENDED_LOG_N, Params::RECOMMENDED_P)?;
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

// pub fn encrypt_key(key: &Keystore, password: &str) -> io::Result<Vec<u8>> {
//     let key_bytes = password_to_key(password);
//     let aes_key = Key::<Aes256Gcm>::from_slice(&key_bytes);
//
//     let cipher = Aes256Gcm::new(aes_key);
//
//     let plaintext = bincode::serialize(key).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Serialization error"))?;
//     let ciphertext = cipher.encrypt(Nonce::from_slice(&key.nonce), plaintext.as_ref())
//         .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Encryption error"))?;
//
//     let mut encrypted_data = bincode::serialize(&key)
//         .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Serialization error"))?;
//     encrypted_data.extend_from_slice(&ciphertext);
//
//     Ok(encrypted_data)
// }

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
    let keystore: Keystore = decrypt_key(&key_store_contents, password)?;
    SecretKey::from_bytes(FieldBytes::from_slice(&keystore.private_key)).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid private key"))
}

// pub fn decrypt_key(data: &[u8], password: &str) -> io::Result<Keystore> {
//     let key: Keystore = bincode::deserialize(data).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Deserialization error"))?;
//     let key_bytes = password_to_key(password);
//     let aes_key = Key::<Aes256Gcm>::from_slice(&key_bytes);
//
//     let cipher = Aes256Gcm::new(aes_key);
//     let nonce = Nonce::from_slice(&key.nonce);
//
//     let ciphertext_start = bincode::serialized_size(&key).unwrap() as usize;
//     let ciphertext = &data[ciphertext_start..];
//
//     let plaintext = cipher.decrypt(nonce, ciphertext)
//         .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Decryption error"))?;
//     let keystore: Keystore = bincode::deserialize(&plaintext).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Deserialization error"))?;
//
//     Ok(keystore)
// }

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
