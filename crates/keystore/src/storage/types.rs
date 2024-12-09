use crate::error::Error;
use erased_serde::{Deserializer, Serialize as ErasedSerialize, Serializer};
use gadget_std::any::TypeId;
use gadget_std::fmt::Debug;
use serde::{Deserialize, Serialize};

/// A serializable key pair entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPairEntry {
    pub public_bytes: Vec<u8>,
    pub secret_bytes: Vec<u8>,
}

/// A serializable public key entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicKeyEntry {
    pub bytes: Vec<u8>,
}

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StorageStats {
    pub total_keys: usize,
    pub total_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredKey {
    pub type_id: TypeId,
    pub public_key: Box<dyn ErasedSerialize + Send + Sync>,
    pub secret_key: Box<dyn ErasedSerialize + Send + Sync>,
}
