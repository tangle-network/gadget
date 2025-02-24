use alloy_primitives::FixedBytes;
use alloy_provider::{
    PendingTransactionBuilder, PendingTransactionError, Provider, ProviderBuilder,
};
use alloy_rpc_types_eth::TransactionReceipt;
use alloy_transport::TransportErrorKind;
use url::Url;

/// Wait for a transaction to finish and return its receipt.
///
/// # Arguments
///
/// `rpc_url` - The RPC URL.
/// `tx_hash` - The hash of the transaction.
///
/// # Returns
///
/// A [`TransportResult`] containing the transaction hash.
pub async fn wait_transaction(
    rpc_url: &str,
    tx_hash: FixedBytes<32>,
) -> Result<TransactionReceipt, PendingTransactionError> {
    let url = Url::parse(rpc_url).map_err(|_| TransportErrorKind::custom_str("Invalid RPC URL"))?;
    let provider = ProviderBuilder::new().on_http(url);
    let pending_tx = PendingTransactionBuilder::new(provider.root().clone(), tx_hash);
    pending_tx.get_receipt().await
}
