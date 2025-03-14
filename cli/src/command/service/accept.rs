use color_eyre::Result;
use dialoguer::console::style;
use gadget_chain_setup::tangle::transactions::get_security_commitment;
use gadget_clients::tangle::client::OnlineClient;
use gadget_crypto::sp_core::SpSr25519;
use gadget_crypto::tangle_pair_signer::TanglePairSigner;
use gadget_keystore::backends::Backend;
use gadget_keystore::{Keystore, KeystoreConfig};
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::types::Asset;

use crate::wait_for_in_block_success;

/// Accepts a service request.
///
/// # Arguments
///
/// * `ws_rpc_url` - WebSocket RPC URL for the Tangle Network
/// * `_min_exposure_percent` - Minimum exposure percentage (currently unused)
/// * `_max_exposure_percent` - Maximum exposure percentage (currently unused)
/// * `restaking_percent` - Percentage of stake to restake
/// * `keystore_uri` - URI for the keystore
/// * `request_id` - ID of the request to accept
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
pub async fn accept_request(
    ws_rpc_url: String,
    _min_exposure_percent: u8,
    _max_exposure_percent: u8,
    restaking_percent: u8,
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

    let native_security_commitments =
        vec![get_security_commitment(Asset::Custom(0), restaking_percent)];

    println!(
        "{}",
        style(format!("Preparing to accept request ID: {}", request_id)).cyan()
    );
    let call = tangle_subxt::tangle_testnet_runtime::api::tx()
        .services()
        .approve(request_id, native_security_commitments);

    println!(
        "{}",
        style(format!(
            "Submitting Service Approval for request ID: {}",
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
            "Service Approval for request ID: {} submitted successfully",
            request_id
        ))
        .green()
    );
    Ok(())
}
