use super::types::{EcdsaRemoteSigner, RemoteConfig};
use crate::{
    error::Error,
    key_types::k256_ecdsa::{K256Ecdsa, K256Signature, K256VerifyingKey},
};
use alloy_primitives::keccak256;
use alloy_signer_aws::AwsSigner;
use aws_config::{BehaviorVersion, Region};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AwsKeyConfig {
    pub key_id: String,
    pub region: String,
    pub chain_id: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AwsRemoteSignerConfig {
    pub keys: Vec<AwsKeyConfig>,
}

impl From<RemoteConfig> for AwsRemoteSignerConfig {
    fn from(config: RemoteConfig) -> Self {
        match config {
            RemoteConfig::Aws { keys } => Self { keys },
            _ => panic!("Invalid config"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct AwsKeyInstance {
    signer: AwsSigner,
    chain_id: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct AwsRemoteSigner {
    signers: BTreeMap<(String, Option<u64>), AwsKeyInstance>,
}

impl AwsRemoteSigner {
    pub async fn new(config: AwsRemoteSignerConfig) -> Result<Self, Error> {
        let mut signers = BTreeMap::new();

        for key_config in config.keys {
            let aws_config = aws_config::defaults(BehaviorVersion::latest())
                .region(Region::new(key_config.region))
                .load()
                .await;
            let client = aws_sdk_kms::Client::new(&aws_config);

            let signer = AwsSigner::new(client, key_config.key_id.clone(), key_config.chain_id)
                .await
                .map_err(|e| Error::Other(e.to_string()))?;

            signers.insert(
                (key_config.key_id, key_config.chain_id),
                AwsKeyInstance {
                    signer,
                    chain_id: key_config.chain_id,
                },
            );
        }

        Ok(Self { signers })
    }
}

#[async_trait::async_trait]
impl EcdsaRemoteSigner<K256Ecdsa> for AwsRemoteSigner {
    type Public = K256VerifyingKey;
    type Signature = K256Signature;
    type KeyId = String;
    type Config = AwsRemoteSignerConfig;

    async fn build(config: RemoteConfig) -> Result<Self, Error> {
        Self::new(config.into()).await
    }

    async fn get_public_key(
        &self,
        key_id: &Self::KeyId,
        chain_id: Option<u64>,
    ) -> Result<Self::Public, Error> {
        // Find signer for the given key ID
        let signer = self
            .signers
            .get(&(key_id.clone(), chain_id))
            .ok_or_else(|| Error::Other(format!("No signer found for key ID {:?}", key_id)))?;

        Ok(K256VerifyingKey(
            signer
                .signer
                .get_pubkey()
                .await
                .map_err(|e| Error::RemoteKeyFetchFailed(e.to_string()))?,
        ))
    }

    async fn iter_public_keys(&self, chain_id: Option<u64>) -> Result<Vec<Self::Public>, Error> {
        let mut public_keys = Vec::new();
        for ((_, _), signer) in &self.signers {
            // Skip if chain_id is Some and doesn't match
            if let Some(chain_id) = chain_id {
                if signer.chain_id != Some(chain_id) {
                    continue;
                }
            }

            let pk = signer
                .signer
                .get_pubkey()
                .await
                .map_err(|e| Error::RemoteKeyFetchFailed(e.to_string()))?;
            public_keys.push(K256VerifyingKey(pk));
        }
        Ok(public_keys)
    }

    async fn get_key_id_from_public_key(
        &self,
        public_key: &Self::Public,
        chain_id: Option<u64>,
    ) -> Result<Self::KeyId, Error> {
        for ((key_id, _), signer) in &self.signers {
            // Skip if chain_id is Some and doesn't match
            if let Some(chain_id) = chain_id {
                if signer.chain_id != Some(chain_id) {
                    continue;
                }
            }

            let pk = signer
                .signer
                .get_pubkey()
                .await
                .map_err(|e| Error::RemoteKeyFetchFailed(e.to_string()))?;

            if pk == public_key.0 {
                return Ok(key_id.clone());
            }
        }

        Err(Error::Other("Key not found".to_string()))
    }

    async fn sign_message_with_key_id(
        &self,
        message: &[u8],
        key_id: &Self::KeyId,
        chain_id: Option<u64>,
    ) -> Result<Self::Signature, Error> {
        let digest = keccak256(message);

        // Find signer for the given key ID
        let signer = self
            .signers
            .get(&(key_id.clone(), chain_id))
            .ok_or_else(|| Error::Other(format!("No signer found for key ID {:?}", key_id)))?;

        Ok(K256Signature(
            signer
                .signer
                .sign_digest(&digest)
                .await
                .map_err(|e| Error::SignatureFailed(e.to_string()))?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use k256::ecdsa::signature::Verifier;

    #[tokio::test]
    #[ignore] // Requires AWS credentials
    async fn test_aws_signer() {
        let config = AwsRemoteSignerConfig {
            keys: vec![AwsKeyConfig {
                key_id: std::env::var("AWS_KMS_KEY_ID").expect("AWS_KMS_KEY_ID not set"),
                region: std::env::var("AWS_REGION").expect("AWS_REGION not set"),
                chain_id: Some(1),
            }],
        };

        let signer = AwsRemoteSigner::new(config).await.unwrap();
        let message = b"test message";

        // Get first signer's key name
        let ((key_name, _), _) = signer.signers.iter().next().unwrap();
        let key_id = key_name.clone();

        let signature = signer
            .sign_message_with_key_id(message, &key_id, Some(1))
            .await
            .unwrap();
        let pk = signer.get_public_key(&key_id, Some(1)).await.unwrap();

        assert!(pk.0.verify(message, &signature.0).is_ok());
    }
}
