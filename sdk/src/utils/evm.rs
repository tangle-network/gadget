use crate::events_watcher::Error;
use crate::info;
use crate::keystore::TanglePairSigner;
use alloy_network::EthereumWallet;
use alloy_primitives::{Address, Bytes, FixedBytes, U256};
use alloy_provider::{Provider, ProviderBuilder, RootProvider, WsConnect};
use alloy_transport::BoxTransport;
use eigensdk::client_avsregistry::writer::AvsRegistryChainWriter;
use eigensdk::client_elcontracts::reader::ELChainReader;
use eigensdk::client_elcontracts::writer::{ELChainWriter, Operator};
use eigensdk::crypto_bls::BlsKeyPair;
use eigensdk::logging::get_test_logger;
use lazy_static::lazy_static;
use rand::random;

lazy_static! {
    /// 1 day
    pub static ref SIGNATURE_EXPIRY: U256 = U256::from(86400);
}

/// Get the provider for a http endpoint
///
/// # Returns
/// - [`RootProvider<BoxTransport>`] - The provider
///
pub fn get_provider_http(http_endpoint: &str) -> RootProvider<BoxTransport> {
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .on_http(http_endpoint.parse().unwrap())
        .root()
        .clone()
        .boxed();

    provider
}

/// Get the provider for a http endpoint with the specified [`Wallet`](EthereumWallet)
///
/// # Returns
/// - [`RootProvider<BoxTransport>`] - The provider
///
pub fn get_wallet_provider_http(
    http_endpoint: &str,
    wallet: EthereumWallet,
) -> RootProvider<BoxTransport> {
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_http(http_endpoint.parse().unwrap())
        .root()
        .clone()
        .boxed();

    provider
}

/// Get the provider for a websocket endpoint
///
/// # Returns
/// - [`RootProvider<BoxTransport>`] - The provider
///
pub async fn get_provider_ws(ws_endpoint: &str) -> RootProvider<BoxTransport> {
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .on_ws(WsConnect::new(ws_endpoint))
        .await
        .unwrap()
        .root()
        .clone()
        .boxed();

    provider
}

/// Get the slasher address from the `DelegationManager` contract
///
/// # Returns
/// - [`Address`] - The slasher address
///
/// # Errors
/// - [`Error::AlloyContract`] - If the call to the contract fails (i.e. the contract doesn't exist at the given address)
///
pub async fn get_slasher_address(
    delegation_manager_addr: Address,
    http_endpoint: &str,
) -> Result<Address, Error> {
    let provider = get_provider_http(http_endpoint);
    let delegation_manager =
        eigensdk::utils::binding::DelegationManager::new(delegation_manager_addr, provider);
    delegation_manager
        .slasher()
        .call()
        .await
        .map(|a| a._0)
        .map_err(Error::AlloyContract)
}

/// Register the account of the given ECDSA pair as an operator on EigenLayer.
///
/// # Returns
/// - [`FixedBytes<32>`] - The transaction hash on success, otherwise an error
///
/// # Errors
/// - [`Error::ElContract`] - If the registration call to the contract itself fails
/// - [`Error::Keys`] - If the key pair fails to be converted for Alloy
pub async fn register_as_operator(
    delegation_manager_addr: Address,
    avs_directory_addr: Address,
    strategy_manager_addr: Address,
    http_endpoint: &str,
    ecdsa_pair: TanglePairSigner<sp_core::ecdsa::Pair>,
) -> Result<FixedBytes<32>, Error> {
    let eigen_logger = get_test_logger();
    let ecdsa_alloy_pair = ecdsa_pair.alloy_key()?;
    let pvt_key = hex::encode(ecdsa_alloy_pair.to_bytes().0);

    let slasher_addr = get_slasher_address(delegation_manager_addr, http_endpoint).await?;

    // A new ElChainReader instance
    let el_chain_reader = ELChainReader::new(
        eigen_logger.clone(),
        slasher_addr,
        delegation_manager_addr,
        avs_directory_addr,
        http_endpoint.to_string(),
    );
    // A new ElChainWriter instance
    let el_writer = ELChainWriter::new(
        delegation_manager_addr,
        strategy_manager_addr,
        el_chain_reader,
        http_endpoint.to_string(),
        pvt_key.to_string(),
    );

    // Get the operator details
    let operator_addr = ecdsa_pair.alloy_address()?;
    let operator_details = Operator {
        address: operator_addr,
        earnings_receiver_address: operator_addr,
        delegation_approver_address: operator_addr,
        staker_opt_out_window_blocks: 50400u32,
        metadata_url: Some(
            "https://github.com/webb-tools/eigensdk-rs/blob/main/test-utils/metadata.json"
                .to_string(),
        ), // TODO: Metadata should be from a Variable
    };

    // Register the address as operator in delegation manager
    el_writer
        .register_as_operator(operator_details)
        .await
        .map_err(Error::ElContract)
}

/// Register the account of the given ECDSA pair with the AVS registry coordinator.
///
/// # Returns
/// - [`FixedBytes<32>`] - The transaction hash on success, otherwise an error
///
/// # Errors
/// - [`Error::AvsRegistry`] - If the registration call to the contract itself fails
pub async fn register_with_avs_registry_coordinator(
    http_endpoint: &str,
    ecdsa_pair: TanglePairSigner<sp_core::ecdsa::Pair>,
    bls_key_pair: BlsKeyPair,
    quorum_nums: Bytes,
    registry_coordinator_addr: Address,
    operator_state_retriever_addr: Address,
) -> Result<FixedBytes<32>, Error> {
    let eigen_logger = get_test_logger();
    let ecdsa_alloy_pair = ecdsa_pair.alloy_key()?;
    let pvt_key = hex::encode(ecdsa_alloy_pair.to_bytes().0);

    // Calculate the signature expiry
    let now = std::time::SystemTime::now();
    let sig_expiry = now
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| U256::from(d.as_secs()) + *SIGNATURE_EXPIRY)
        .unwrap_or_else(|| {
            info!("System time seems to be before the UNIX epoch.");
            *SIGNATURE_EXPIRY
        });

    // Random salt
    let salt: FixedBytes<32> = FixedBytes::from(random::<[u8; 32]>());

    // Register the operator in the AVS registry coordinator
    let avs_registry_writer = AvsRegistryChainWriter::build_avs_registry_chain_writer(
        eigen_logger.clone(),
        http_endpoint.to_string(),
        pvt_key.to_string(),
        registry_coordinator_addr,
        operator_state_retriever_addr,
    )
    .await
    .expect("Failed to build AVS Registry Writer");
    avs_registry_writer
        .register_operator_in_quorum_with_avs_registry_coordinator(
            bls_key_pair,
            salt,
            sig_expiry,
            quorum_nums,
            http_endpoint.to_string(),
        )
        .await
        .map_err(Error::AvsRegistry)
}
