use crate::Error;
use alloy_network::EthereumWallet;
use alloy_primitives::{Address, U256};
use alloy_provider::{Provider, ProviderBuilder, RootProvider, WsConnect};
use alloy_transport::BoxTransport;

/// 1 day
pub const SIGNATURE_EXPIRY: U256 = U256::from_limbs([86400, 0, 0, 0]);

/// Get the provider for a http endpoint
///
/// # Returns
/// - [`RootProvider<BoxTransport>`] - The provider
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
pub async fn get_slasher_address(
    delegation_manager_addr: Address,
    http_endpoint: &str,
) -> Result<Address, Error> {
    let provider = get_provider_http(http_endpoint);
    let delegation_manager =
        eigensdk::utils::delegationmanager::DelegationManager::DelegationManagerInstance::new(
            delegation_manager_addr,
            provider,
        );
    delegation_manager
        .slasher()
        .call()
        .await
        .map(|a| a._0)
        .map_err(|err| Error::Client(err.to_string()))
}
