use std::sync::Arc;

use crate::anvil::{Anvil, AnvilInstance};
use alloy_provider::ProviderBuilder;
use alloy_signer_local::PrivateKeySigner;
use alloy_transport_ws::WsConnect;
use eigen_contracts::*;
use futures::prelude::*;
// use avs::IncredibleSquaringServiceManager;
use eigen_contracts::*;
use gadget_common::subxt_signer::bip39::rand_core::OsRng;
use k256::ecdsa::VerifyingKey;
use k256::elliptic_curve::SecretKey;

type EthersClient = ethers::providers::Provider<ethers::providers::Http>;
type SignerEthersClient =
    ethers::middleware::SignerMiddleware<EthersClient, ethers::signers::LocalWallet>;

pub struct LocalEvmChain {
    chain_id: u32,
    name: String,
    client: Arc<SignerEthersClient>,
    anvil_node_handle: AnvilInstance,
}

impl LocalEvmChain {
    pub fn new(chain_id: u32, name: String, port: Option<u16>) -> Self {
        let anvil_node_handle = Self::spawn_anvil_node(chain_id, None, port);
        let secret_key = anvil_node_handle.keys()[0].clone();
        let signer = ethers::signers::LocalWallet::from(secret_key).with_chain_id(chain_id);
        let provider = ethers::providers::Provider::<ethers::providers::Http>::try_from(
            anvil_node_handle.endpoint(),
        )
        .unwrap();
        let client = SignerEthersClient::new(provider, signer);
        // start the node
        Self {
            chain_id,
            name,
            client: Arc::new(client),
            anvil_node_handle,
        }
    }

    pub fn new_with_chain_state(
        chain_id: u32,
        name: String,
        state_dir: &std::path::Path,
        port: Option<u16>,
    ) -> Self {
        let anvil_node_handle = Self::spawn_anvil_node(chain_id, Some(state_dir), port);
        let secret_key = anvil_node_handle.keys()[0].clone();
        let signer = ethers::signers::LocalWallet::from(secret_key).with_chain_id(chain_id);
        let provider = ethers::providers::Provider::<ethers::providers::Http>::try_from(
            anvil_node_handle.endpoint(),
        )
        .unwrap();
        let client = SignerEthersClient::new(provider, signer);
        Self {
            chain_id,
            name,
            client: Arc::new(client),
            anvil_node_handle,
        }
    }

    pub fn chain_id(&self) -> u32 {
        self.chain_id
    }

    /// Returns typed EVM chain id
    pub fn typed_chain_id(&self) -> u64 {
        TypedChainId::Evm(self.chain_id).chain_id()
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn keys(&self) -> &[ethers::core::k256::SecretKey] {
        self.anvil_node_handle.keys()
    }

    pub fn addresses(&self) -> &[ethers::types::Address] {
        self.anvil_node_handle.addresses()
    }

    pub fn client(&self) -> Arc<SignerEthersClient> {
        self.client.clone()
    }

    /// Returns the HTTP endpoint of this instance
    pub fn endpoint(&self) -> String {
        self.anvil_node_handle.endpoint()
    }

    /// Returns the Websocket endpoint of this instance
    pub fn ws_endpoint(&self) -> String {
        self.anvil_node_handle.ws_endpoint()
    }

    pub fn shutdown(self) {
        drop(self.anvil_node_handle);
    }

    /// Deploy a new ERC20 token.
    ///
    /// # Errors
    ///
    /// This function will return an error if the deployment fails.
    pub async fn deploy_token(
        &self,
        name: String,
        symbol: String,
    ) -> Result<ERC20PresetMinterPauserContract<SignerEthersClient>> {
        ERC20PresetMinterPauserContract::deploy(self.client.clone(), (name, symbol))?
            .confirmations(0usize)
            .send()
            .map_err(Into::into)
            .await
    }

    /// Deploy Token wrapper handler.
    ///
    /// # Errors
    ///
    /// This function will return an error if the deployment fails.
    pub async fn deploy_token_wrapper_handler(
        &self,
        bridge_contract_address: ethers::types::Address,
        initial_resource_ids: Vec<webb_proposals::ResourceId>,
        initial_contract_addresses: Vec<ethers::types::Address>,
    ) -> Result<TokenWrapperHandlerContract<SignerEthersClient>> {
        let initial_r_ids = initial_resource_ids
            .iter()
            .map(webb_proposals::ResourceId::to_bytes)
            .collect::<Vec<_>>();

        TokenWrapperHandlerContract::deploy(
            self.client.clone(),
            (
                bridge_contract_address,
                initial_r_ids,
                initial_contract_addresses,
            ),
        )?
        .confirmations(0usize)
        .send()
        .map_err(Into::into)
        .await
    }

    /// Deploy fungible token wrapper contract.
    ///
    /// # Errors
    ///
    /// This function will return an error if the deployment fails.
    pub async fn deploy_fungible_token_wrapper(
        &self,
        name: String,
        symbol: String,
        fee_percentage: u16,
        fee_recipient: ethers::types::Address,
        token_handler: ethers::types::Address,
        limit: U256,
        is_native_allowed: bool,
        admin: ethers::types::Address,
    ) -> Result<FungibleTokenWrapperContract<SignerEthersClient>> {
        let contract = FungibleTokenWrapperContract::deploy(self.client.clone(), (name, symbol))?
            .confirmations(0usize)
            .send()
            .await?;

        contract
            .initialize(
                fee_percentage,
                fee_recipient,
                token_handler,
                limit,
                is_native_allowed,
                admin,
            )
            .call()
            .await?;

        Ok(contract)
    }

    /// Deploy a new Signature Bridge.
    ///
    /// # Errors
    ///
    /// This function will return an error if the deployment fails.
    pub async fn deploy_signature_bridge(
        &self,
        initial_governor: ethers::types::Address,
        nonce: u32,
    ) -> Result<SignatureBridgeContract<SignerEthersClient>> {
        SignatureBridgeContract::deploy(self.client.clone(), (initial_governor, nonce))?
            .confirmations(0usize)
            .send()
            .map_err(Into::into)
            .await
    }

    pub async fn deploy_poseidon_hasher(
        &self,
    ) -> Result<PoseidonHasherContract<SignerEthersClient>> {
        let t3 = PoseidonT3Contract::deploy(self.client.clone(), ())?
            .confirmations(0usize)
            .send()
            .await?;
        let t4 = PoseidonT4Contract::deploy(self.client.clone(), ())?
            .confirmations(0usize)
            .send()
            .await?;
        let t6 = PoseidonT6Contract::deploy(self.client.clone(), ())?
            .confirmations(0usize)
            .send()
            .await?;

        let hasher_factory = poseidon_hasher_factory::create(
            t3.address(),
            t4.address(),
            t6.address(),
            self.client.clone(),
        )?;
        let contract = hasher_factory
            .deploy(())?
            .confirmations(0usize)
            .send()
            .await?;
        let hasher = PoseidonHasherContract::new(contract.address(), self.client.clone());
        Ok(hasher)
    }

    pub async fn deploy_vanchor_verifier(
        &self,
    ) -> Result<VAnchorVerifierContract<SignerEthersClient>> {
        let v22 = Verifier2x2Contract::deploy(self.client(), ())?
            .confirmations(0usize)
            .send()
            .await?;
        let v216 = Verifier2x16Contract::deploy(self.client(), ())?
            .confirmations(0usize)
            .send()
            .await?;
        let v82 = Verifier8x2Contract::deploy(self.client(), ())?
            .confirmations(0usize)
            .send()
            .await?;
        let v816 = Verifier8x16Contract::deploy(self.client(), ())?
            .confirmations(0usize)
            .send()
            .await?;
        VAnchorVerifierContract::deploy(
            self.client(),
            (v22.address(), v216.address(), v82.address(), v816.address()),
        )?
        .confirmations(0usize)
        .send()
        .map_err(Into::into)
        .await
    }

    /// Deploy a new Anchor Handler.
    ///
    /// # Errors
    ///
    /// This function will return an error if the deployment fails.
    pub async fn deploy_anchor_handler(
        &self,
        bridge_contract_address: ethers::types::Address,
        initial_resource_ids: Vec<webb_proposals::ResourceId>,
        initial_contract_addresses: Vec<ethers::types::Address>,
    ) -> Result<AnchorHandlerContract<SignerEthersClient>> {
        let initial_r_ids = initial_resource_ids
            .iter()
            .map(webb_proposals::ResourceId::to_bytes)
            .collect::<Vec<_>>();
        AnchorHandlerContract::deploy(
            self.client.clone(),
            (
                bridge_contract_address,
                initial_r_ids,
                initial_contract_addresses,
            ),
        )?
        .confirmations(0usize)
        .send()
        .map_err(Into::into)
        .await
    }

    /// Deploy a new Treasury Handler.
    ///
    /// # Errors
    ///
    /// This function will return an error if the deployment fails.
    pub async fn deploy_treasury_handler(
        &self,
        bridge_contract_address: ethers::types::Address,
        initial_resource_ids: Vec<webb_proposals::ResourceId>,
        initial_contract_addresses: Vec<ethers::types::Address>,
    ) -> Result<TreasuryHandlerContract<SignerEthersClient>> {
        let initial_r_ids = initial_resource_ids
            .iter()
            .map(webb_proposals::ResourceId::to_bytes)
            .collect::<Vec<_>>();
        TreasuryHandlerContract::deploy(
            self.client.clone(),
            (
                bridge_contract_address,
                initial_r_ids,
                initial_contract_addresses,
            ),
        )?
        .confirmations(0usize)
        .send()
        .map_err(Into::into)
        .await
    }

    /// Deploy a new Treasury.
    ///
    /// # Errors
    ///
    /// This function will return an error if the deployment fails.
    pub async fn deploy_treasury(
        &self,
        treasury_handler_contract_address: ethers::types::Address,
    ) -> Result<TreasuryContract<SignerEthersClient>> {
        TreasuryContract::deploy(self.client.clone(), treasury_handler_contract_address)?
            .confirmations(0usize)
            .send()
            .map_err(Into::into)
            .await
    }

    /// Deploy a new VAnchorTree.
    ///
    /// # Errors
    ///
    /// This function will return an error if the deployment fails.
    pub async fn deploy_vanchor_tree(
        &self,
        verifier: ethers::types::Address,
        merkle_tree_levels: u32,
        hasher: ethers::types::Address,
        handler: ethers::types::Address,
        token: ethers::types::Address,
        max_edges: u8,
        minimal_withdrawal_amount: ethers::types::U256,
        maximum_deposit_amount: ethers::types::U256,
    ) -> Result<VAnchorTreeContract<SignerEthersClient>> {
        let client = self.client.clone();
        let vanchor_encode_inputs = VAnchorEncodeInputsContract::deploy(client.clone(), ())?
            .confirmations(0usize)
            .send()
            .await?;
        let contract =
            vanchor_tree_factory::create(vanchor_encode_inputs.address(), client.clone())?
                .deploy((
                    verifier,
                    merkle_tree_levels,
                    hasher,
                    handler,
                    token,
                    max_edges,
                ))?
                .confirmations(0usize)
                .send()
                .await?;

        let contract = VAnchorTreeContract::new(contract.address(), client);

        contract
            .initialize(minimal_withdrawal_amount, maximum_deposit_amount)
            .send()
            .await?;
        Ok(contract)
    }

    fn spawn_anvil_node(
        chain_id: u32,
        state_dir: Option<&std::path::Path>,
        port: Option<u16>,
    ) -> AnvilInstance {
        let port = if port.is_some() {
            port.unwrap()
        } else {
            crate::random_port::random_port()
        };
        let mut anvil = Anvil::new()
            .port(port)
            .chain_id(chain_id)
            .arg("--accounts")
            .arg("20");
        if let Some(p) = state_dir {
            anvil = anvil
                .arg("--dump-state")
                .arg(p.to_string_lossy())
                .arg("--state-interval")
                .arg("1");
            if p.join("state.json").exists() {
                anvil = anvil.arg("--load-state").arg(p.to_string_lossy());
            };
        };
        anvil.spawn()
    }
}

#[cfg(test)]
mod tests {
    use webb::evm::ethers::types::U256;

    use super::*;

    #[tokio::test]
    async fn should_be_able_to_deploy_token() -> Result<()> {
        let chain = LocalEvmChain::new(1337, String::from("Hermes"), Some(9955u16));
        let token = chain
            .deploy_token(String::from("Test"), String::from("TST"))
            .await?;
        let name = token.name().call().await?;
        assert_eq!(name, "Test");
        let symbol = token.symbol().call().await?;
        assert_eq!(symbol, "TST");
        chain.shutdown();
        Ok(())
    }

    #[tokio::test]
    async fn should_be_able_to_deploy_hasher() -> Result<()> {
        let chain = LocalEvmChain::new(5001, String::from("Hermes"), None);
        let hasher = chain.deploy_poseidon_hasher().await?;
        let hash = hasher
            .hash_left_right(U256::from(1), U256::from(2))
            .call()
            .await?;
        let expected_result = U256::from_str_radix(
            "115cc0f5e7d690413df64c6b9662e9cf2a3617f2743245519e19607a4417189a",
            16,
        );
        assert_eq!(hash, expected_result.unwrap());
        Ok(())
    }

    #[tokio::test]
    async fn should_be_able_to_deploy_vanchor_tree() -> Result<()> {
        let chain = LocalEvmChain::new(5001, String::from("Hermes"), None);
        let initial_governor = ethers::types::Address::zero();
        let bridge = chain.deploy_signature_bridge(initial_governor, 0).await?;
        let verifier = chain.deploy_vanchor_verifier().await?;
        let merkle_tree_levels = 30;
        let hasher = chain.deploy_poseidon_hasher().await?;
        let handler = chain
            .deploy_anchor_handler(bridge.address(), vec![], vec![])
            .await?;
        let token = chain
            .deploy_token(String::from("Test"), String::from("TST"))
            .await?;
        let vanchor = chain
            .deploy_vanchor_tree(
                verifier.address(),
                merkle_tree_levels,
                hasher.address(),
                handler.address(),
                token.address(),
                2,
                0.into(),
                u64::MAX.into(),
            )
            .await?;
        // hasher on the vanchor tree should be the same as the one we deployed
        let hash = vanchor
            .hash_left_right(U256::from(1), U256::from(2))
            .call()
            .await?;
        let expected_result = U256::from_str_radix(
            "115cc0f5e7d690413df64c6b9662e9cf2a3617f2743245519e19607a4417189a",
            16,
        );
        assert_eq!(hash, expected_result.unwrap());
        Ok(())
    }

    #[tokio::test]
    async fn should_load_old_state() -> Result<()> {
        let state = tempfile::Builder::new()
            .prefix("evm-test-utils")
            .tempdir()?;
        assert!(state.path().is_dir());

        let chain =
            LocalEvmChain::new_with_chain_state(5001, String::from("Hermes"), state.path(), None);
        let token = chain
            .deploy_token(String::from("Test"), String::from("TST"))
            .await?;
        let name = token.name().call().await?;
        assert_eq!(name, "Test");
        chain.shutdown();
        let chain =
            LocalEvmChain::new_with_chain_state(5001, String::from("Hermes"), state.path(), None);
        let token = ERC20PresetMinterPauserContract::new(token.address(), chain.client());
        let name = token.name().call().await?;
        assert_eq!(name, "Test");
        chain.shutdown();
        Ok(())
    }
}
