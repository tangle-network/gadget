use alloy_primitives::Address;
use alloy_provider::{Provider, ProviderBuilder, RootProvider};
use alloy_signer_local::PrivateKeySigner;
use blueprint_sdk::testing::utils::anvil::keys::ANVIL_PRIVATE_KEYS;
use eigensdk::client_avsregistry::reader::AvsRegistryChainReader;
use eigensdk::crypto_bls::BlsKeyPair;
use eigensdk::services_avsregistry::chaincaller::AvsRegistryServiceChainCaller;
use eigensdk::services_blsaggregation::bls_agg::BlsAggregatorService;
use eigensdk::services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceInMemory;

pub fn get_provider_http(http_endpoint: &str) -> RootProvider {
    let provider = ProviderBuilder::new()
        .on_http(http_endpoint.parse().unwrap())
        .root()
        .clone();

    provider
}

pub type BlsAggServiceInMemory = BlsAggregatorService<
    AvsRegistryServiceChainCaller<AvsRegistryChainReader, OperatorInfoServiceInMemory>,
>;

pub fn get_wallet_provider_http(
    http_endpoint: &str,
    signer: alloy_signer_local::PrivateKeySigner,
) -> RootProvider {
    let wallet = alloy_network::EthereumWallet::new(signer);
    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .on_http(http_endpoint.parse().unwrap())
        .root()
        .clone();

    provider
}

#[derive(Clone, Debug)]
pub struct Keystore {
    ecdsa_key: String,
    bls_keypair: BlsKeyPair,
}

impl Default for Keystore {
    fn default() -> Self {
        // Use first Anvil private key
        let ecdsa_key = ANVIL_PRIVATE_KEYS[0].to_string();

        // Hardcoded BLS private key for testing
        let bls_private_key =
            "1371012690269088913462269866874713266643928125698382731338806296762673180359922";
        let bls_keypair = BlsKeyPair::new(bls_private_key.to_string()).expect("Invalid BLS key");

        Self {
            ecdsa_key,
            bls_keypair,
        }
    }
}

impl Keystore {
    pub fn ecdsa_private_key(&self) -> &str {
        &self.ecdsa_key
    }

    pub fn ecdsa_address(&self) -> Address {
        let private_key: PrivateKeySigner = self.ecdsa_key.parse().unwrap();
        private_key.address()
    }

    pub fn bls_keypair(&self) -> &BlsKeyPair {
        &self.bls_keypair
    }
}
