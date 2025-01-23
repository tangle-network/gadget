use alloy_network::EthereumWallet;
use alloy_primitives::U256;
use alloy_provider::{Provider, ProviderBuilder, RootProvider, WsConnect};
use alloy_signer_local::PrivateKeySigner;
use alloy_transport::BoxTransport;
use gadget_std::str::FromStr;
use url::Url;

/// 1 day
pub const SIGNATURE_EXPIRY: U256 = U256::from_limbs([86400, 0, 0, 0]);

/// Get the provider for a http endpoint
///
/// # Returns
/// - [`RootProvider<BoxTransport>`] - The provider
///
/// # Panics
/// - If the provided http endpoint is not a valid URL
#[must_use]
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
/// # Panics
/// - If the provided http endpoint is not a valid URL
#[must_use]
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
/// # Panics
/// - If the provided websocket endpoint is not a valid URL
#[must_use]
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

#[allow(clippy::type_complexity)]
/// Get the provider for an http endpoint with the [`Wallet`](EthereumWallet) for the specified private key
///
/// # Returns
/// - [`RootProvider<BoxTransport>`] - The provider
///
/// # Panics
/// - If the provided http endpoint is not a valid URL
#[must_use]
pub fn get_provider_from_signer(key: &str, rpc_url: &str) -> RootProvider<BoxTransport> {
    let signer = PrivateKeySigner::from_str(key).expect("wrong key ");
    let wallet = EthereumWallet::from(signer);
    let url = Url::parse(rpc_url).expect("Wrong rpc url");
    ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet.clone())
        .on_http(url)
        .root()
        .clone()
        .boxed()
}
