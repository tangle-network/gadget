use color_eyre::Result;
use dialoguer::console::style;
use gadget_clients::tangle::client::OnlineClient;
use gadget_crypto::sp_core::SpSr25519;
use gadget_crypto::tangle_pair_signer::TanglePairSigner;
use gadget_keystore::{Keystore, KeystoreConfig};
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::sp_arithmetic::per_things::Percent;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::types::{
    Asset, AssetSecurityRequirement, MembershipModel,
};
use crate::wait_for_in_block_success;
use tangle_subxt::subxt::utils::AccountId32;
use gadget_keystore::backends::Backend;

/// Requests a service from the Tangle Network.
///
/// # Arguments
///
/// * `ws_rpc_url` - WebSocket RPC URL for the Tangle Network
/// * `blueprint_id` - ID of the blueprint to request
/// * `min_exposure_percent` - Minimum exposure percentage
/// * `max_exposure_percent` - Maximum exposure percentage
/// * `target_operators` - List of target operators
/// * `value` - Value to stake
/// * `keystore_uri` - URI for the keystore
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
pub async fn request_service(
    ws_rpc_url: String,
    blueprint_id: u64,
    min_exposure_percent: u8,
    max_exposure_percent: u8,
    target_operators: Vec<AccountId32>,
    value: u128,
    keystore_uri: String,
    // keystore_password: Option<String>, // TODO: Add keystore password support
) -> Result<()> {
    let client = OnlineClient::from_url(ws_rpc_url.clone()).await?;

    // Get the next service ID before submitting the request
    let next_service_id_storage = tangle_subxt::tangle_testnet_runtime::api::storage()
        .services()
        .next_instance_id();
    let next_service_id = client
        .storage()
        .at_latest()
        .await?
        .fetch_or_default(&next_service_id_storage)
        .await?;

    let config = KeystoreConfig::new().fs_root(keystore_uri.clone());
    let keystore = Keystore::new(config).expect("Failed to create keystore");
    let public = keystore.first_local::<SpSr25519>().unwrap();
    let pair = keystore.get_secret::<SpSr25519>(&public).unwrap();
    let signer = TanglePairSigner::new(pair.0);

    let min_operators = u32::try_from(target_operators.len())
        .map_err(|_| color_eyre::eyre::eyre!("Too many operators"))?;
    let security_requirements = vec![AssetSecurityRequirement {
        asset: Asset::Custom(0),
        min_exposure_percent: Percent(min_exposure_percent),
        max_exposure_percent: Percent(max_exposure_percent),
    }];

    println!(
        "{}",
        style(format!(
            "Preparing service request for blueprint ID: {}",
            blueprint_id
        ))
        .cyan()
    );
    println!(
        "{}",
        style(format!(
            "Target operators: {} (min: {})",
            target_operators.len(),
            min_operators
        ))
        .dim()
    );
    println!(
        "{}",
        style(format!(
            "Exposure range: {}% - {}%",
            min_exposure_percent, max_exposure_percent
        ))
        .dim()
    );

    let call = tangle_subxt::tangle_testnet_runtime::api::tx()
        .services()
        .request(
            None,
            blueprint_id,
            Vec::new(),
            target_operators,
            Vec::default(),
            security_requirements,
            1000,
            Asset::Custom(0),
            value,
            MembershipModel::Fixed { min_operators },
        );

    println!("{}", style("Submitting Service Request...").cyan());
    let res = client
        .tx()
        .sign_and_submit_then_watch_default(&call, &signer)
        .await?;
    wait_for_in_block_success(res).await;

    println!(
        "{}",
        style("Service Request submitted successfully").green()
    );

    println!(
        "{}",
        style(format!("Service ID: {}", next_service_id))
            .green()
            .bold()
    );

    Ok(())
}
