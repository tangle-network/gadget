use crate::error::{Error, Result};

use crate::keystore::Keystore;
use alloy_network::EthereumWallet;
use alloy_primitives::{Address, B256};
use alloy_signer_local::PrivateKeySigner;
use alloy_signer_local::{MnemonicBuilder, coins_bip39::English};
use gadget_crypto::{BytesEncoding, KeyType};
use gadget_std::string::ToString;
use serde::de::DeserializeOwned;

#[async_trait::async_trait]
pub trait EvmBackend: Send + Sync {
    /// Create an EVM wallet from a private key
    ///
    /// # Errors
    ///
    /// * If the `private_key` is invalid
    fn create_wallet_from_private_key(&self, private_key: &[u8]) -> Result<EthereumWallet>;

    /// Create an EVM wallet from a string seed
    ///
    /// # Errors
    ///
    /// See [`EvmBackend::create_wallet_from_private_key`]
    fn create_wallet_from_string_seed(&self, seed: &str) -> Result<EthereumWallet>;

    /// Get the EVM address for a public key
    ///
    /// # Errors
    ///
    /// Depending on the backend, this may error when attempting to open and/or read the keystore.
    fn get_address<T: KeyType>(&self, public: &T::Public) -> Result<Address>
    where
        T::Public: DeserializeOwned;

    // Optional mnemonic features
    /// Create a wallet from a mnemonic string
    ///
    /// # Errors
    ///
    /// * If the mnemonic is invalid
    fn create_wallet_from_mnemonic(&self, mnemonic: &str) -> Result<EthereumWallet>;

    /// Create a wallet from a mnemonic string with a derivation path
    ///
    /// # Errors
    ///
    /// * If the mnemonic is invalid
    fn create_wallet_from_mnemonic_with_path(
        &self,
        mnemonic: &str,
        path: &str,
    ) -> Result<EthereumWallet>;
}

impl EvmBackend for Keystore {
    fn create_wallet_from_private_key(&self, private_key: &[u8]) -> Result<EthereumWallet> {
        if private_key.len() != 32 {
            return Err(Error::InvalidSeed("Invalid private key length".to_string()));
        }
        let private_key: B256 = private_key.try_into().unwrap_or_default();
        let private_key_signer =
            PrivateKeySigner::from_bytes(&private_key).map_err(|e| Error::Other(e.to_string()))?;
        Ok(EthereumWallet::from(private_key_signer))
    }

    fn create_wallet_from_string_seed(&self, seed: &str) -> Result<EthereumWallet> {
        let seed_bytes = blake3::hash(seed.as_bytes()).as_bytes().to_vec();
        self.create_wallet_from_private_key(&seed_bytes)
    }

    fn get_address<T: KeyType>(&self, public: &T::Public) -> Result<Address>
    where
        T::Public: DeserializeOwned,
    {
        let public_bytes = public.to_bytes();
        Ok(Address::from_slice(&public_bytes[..20]))
    }

    fn create_wallet_from_mnemonic(&self, mnemonic: &str) -> Result<EthereumWallet> {
        let builder: MnemonicBuilder<English> = MnemonicBuilder::default().phrase(mnemonic);
        let signer = builder.build()?;
        let wallet = EthereumWallet::from(signer);
        Ok(wallet)
    }

    fn create_wallet_from_mnemonic_with_path(
        &self,
        mnemonic: &str,
        path: &str,
    ) -> Result<EthereumWallet> {
        let builder: MnemonicBuilder<English> = MnemonicBuilder::default()
            .phrase(mnemonic)
            .derivation_path(path)?;
        let private_key = builder.build()?;
        let wallet = EthereumWallet::from(private_key);
        Ok(wallet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KeystoreConfig;

    #[test]
    fn test_basic_wallet_creation() -> Result<()> {
        let keystore = Keystore::new(KeystoreConfig::new())?;

        // Test private key wallet
        let private_key = [1u8; 32];
        let wallet = keystore.create_wallet_from_private_key(&private_key)?;
        assert_ne!(wallet.default_signer().address(), Address::ZERO);

        // Test seed wallet
        let seed_wallet = keystore.create_wallet_from_string_seed("test seed")?;
        assert_ne!(seed_wallet.default_signer().address(), Address::ZERO);

        Ok(())
    }

    #[test]
    fn test_mnemonic_wallet() -> Result<()> {
        let keystore = Keystore::new(KeystoreConfig::new())?;
        let mnemonic = "test test test test test test test test test test test junk";
        let wallet = keystore.create_wallet_from_mnemonic(mnemonic)?;
        assert_ne!(wallet.default_signer().address(), Address::ZERO);
        Ok(())
    }
}
