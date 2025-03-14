use crate::wait_for_in_block_success;
use color_eyre::Result;
use dialoguer::console::style;
use gadget_clients::tangle::client::OnlineClient;
use gadget_crypto::sp_core::SpSr25519;
use gadget_crypto::tangle_pair_signer::TanglePairSigner;
use gadget_keystore::backends::Backend;
use gadget_keystore::{Keystore, KeystoreConfig};

/// Rejects a service request.
///
/// # Arguments
///
/// * `ws_rpc_url` - WebSocket RPC URL for the Tangle Network
/// * `keystore_uri` - URI for the keystore
/// * `request_id` - ID of the request to reject
///
/// # Errors
///
/// Returns an error if:
/// * Failed to connect to the Tangle Network
/// * Failed to sign or submit the transaction
/// * Transaction failed
///
/// # Panics
///
/// Panics if:
/// * Failed to create keystore
/// * Failed to get keys from keystore
pub async fn reject_request(
    ws_rpc_url: String,
    keystore_uri: String,
    // keystore_password: Option<String>, // TODO: Add keystore password support
    request_id: u64,
) -> Result<()> {
    let client = OnlineClient::from_url(ws_rpc_url.clone()).await?;

    let config = KeystoreConfig::new().fs_root(keystore_uri.clone());
    let keystore = Keystore::new(config).expect("Failed to create keystore");
    let public = keystore.first_local::<SpSr25519>().unwrap();
    let pair = keystore.get_secret::<SpSr25519>(&public).unwrap();
    let signer = TanglePairSigner::new(pair.0);

    println!(
        "{}",
        style(format!("Preparing to reject request ID: {}", request_id)).cyan()
    );
    let call = tangle_subxt::tangle_testnet_runtime::api::tx()
        .services()
        .reject(request_id);

    println!(
        "{}",
        style(format!(
            "Submitting Service Rejection for request ID: {}",
            request_id
        ))
        .cyan()
    );
    let res = client
        .tx()
        .sign_and_submit_then_watch_default(&call, &signer)
        .await?;
    wait_for_in_block_success(res).await;

    println!(
        "{}",
        style(format!(
            "Service Rejection for request ID: {} submitted successfully",
            request_id
        ))
        .green()
    );
    Ok(())
}
