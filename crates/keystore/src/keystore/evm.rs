use crate::{
    error::Error,
    key_types::{KeyType, KeyTypeId},
    storage::RawStorage,
};
use alloy_network::{Ethereum, EthereumWallet};
use alloy_primitives::{Address, B256};
use alloy_signer_local::MnemonicBuilder;
use alloy_signer_local::PrivateKeySigner;
use serde::de::DeserializeOwned;

use super::Keystore;

#[async_trait::async_trait]
pub trait EvmBackend: Send + Sync {
    /// Create an EVM wallet from a private key
    fn create_wallet_from_private_key(&self, private_key: &[u8]) -> Result<EthereumWallet, Error>;

    /// Create an EVM wallet from a string seed
    fn create_wallet_from_string_seed(&self, seed: &str) -> Result<EthereumWallet, Error>;

    /// Get the EVM address for a public key
    fn get_address<T: KeyType>(&self, public: &T::Public) -> Result<Address, Error>
    where
        T::Public: DeserializeOwned;

    // Optional mnemonic features
    fn create_wallet_from_mnemonic(&self, mnemonic: &str) -> Result<EthereumWallet, Error>;

    fn create_wallet_from_mnemonic_with_path(
        &self,
        mnemonic: &str,
        path: &str,
    ) -> Result<EthereumWallet, Error>;

    // Optional YubiHSM features
    fn create_wallet_from_yubihsm(
        &self,
        connector_url: &str,
        auth_key_id: u16,
        password: &str,
        signing_key_id: u16,
    ) -> Result<EthereumWallet, Error>;
}

impl EvmBackend for Keystore {
    fn create_wallet_from_private_key(&self, private_key: &[u8]) -> Result<EthereumWallet, Error> {
        if private_key.len() != 32 {
            return Err(Error::InvalidSeed("Invalid private key length".to_string()));
        }
        let private_key: B256 = private_key.try_into().unwrap_or_default();
        let private_key_signer = PrivateKeySigner::from_bytes(&private_key);
        Ok(EthereumWallet::from(private_key_signer))
    }

    fn create_wallet_from_string_seed(&self, seed: &str) -> Result<EthereumWallet, Error> {
        let seed_bytes = blake3::hash(seed.as_bytes()).as_bytes().to_vec();
        self.create_wallet_from_private_key(&seed_bytes)
    }

    fn get_address<T: KeyType>(&self, public: &T::Public) -> Result<Address, Error>
    where
        T::Public: DeserializeOwned,
    {
        let public_bytes = serde_json::to_vec(public)?;
        Ok(Address::from_slice(&public_bytes[..20]))
    }

    fn create_wallet_from_mnemonic(&self, mnemonic: &str) -> Result<EthereumWallet, Error> {
        let builder = MnemonicBuilder::default().phrase(mnemonic);
        let wallet = builder.build()?;
        Ok(wallet)
    }

    fn create_wallet_from_mnemonic_with_path(
        &self,
        mnemonic: &str,
        path: &str,
    ) -> Result<EthereumWallet, Error> {
        let builder = MnemonicBuilder::default()
            .phrase(mnemonic)
            .derivation_path(path);
        let wallet = builder.build()?;
        Ok(wallet)
    }

    fn create_wallet_from_yubihsm(
        &self,
        connector_url: &str,
        auth_key_id: u16,
        password: &str,
        signing_key_id: u16,
    ) -> Result<EthereumWallet, Error> {
        let signer = YubiHsmSigner::new(connector_url, auth_key_id, password, signing_key_id)?;
        Ok(EthereumWallet::from(signer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_wallet_creation() -> Result<(), Error> {
        let keystore = Keystore::new();

        // Test private key wallet
        let private_key = [1u8; 32];
        let wallet = keystore.create_wallet_from_private_key(&private_key)?;
        assert!(wallet.address() != Address::ZERO);

        // Test seed wallet
        let seed_wallet = keystore.create_wallet_from_string_seed("test seed")?;
        assert!(seed_wallet.address() != Address::ZERO);

        Ok(())
    }

    #[cfg(feature = "evm-mnemonic")]
    #[test]
    fn test_mnemonic_wallet() -> Result<(), Error> {
        let keystore = Keystore::new();
        let mnemonic = "test test test test test test test test test test test junk";
        let wallet = keystore.create_wallet_from_mnemonic(mnemonic)?;
        assert!(wallet.address() != Address::ZERO);
        Ok(())
    }
}
