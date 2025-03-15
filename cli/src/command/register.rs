use blueprint_core::info;
use blueprint_runner::tangle::config::{PriceTargets, decompress_pubkey};
use color_eyre::Result;
use dialoguer::console::style;
use gadget_clients::tangle::client::OnlineClient;
use gadget_crypto::sp_core::SpSr25519;
use gadget_crypto::tangle_pair_signer::TanglePairSigner;
use gadget_keystore::backends::Backend;
use gadget_keystore::{Keystore, KeystoreConfig};
use tangle_subxt::subxt::tx::Signer;
use tangle_subxt::tangle_testnet_runtime::api;

/// Registers a blueprint.
///
/// # Arguments
///
/// * `ws_rpc_url` - WebSocket RPC URL for the Tangle Network
/// * `blueprint_id` - ID of the blueprint to register
/// * `keystore_uri` - URI for the keystore
///
/// # Errors
///
/// Returns an error if:
/// * Failed to connect to the Tangle Network
/// * Failed to sign or submit the transaction
/// * Transaction failed
/// * Missing ECDSA key
///
/// # Panics
///
/// Panics if:
/// * Failed to create keystore
/// * Failed to get keys from keystore
pub async fn register(
    ws_rpc_url: String,
    blueprint_id: u64,
    keystore_uri: String,
    // keystore_password: Option<String>, // TODO: Add keystore password support
) -> Result<()> {
    let client = OnlineClient::from_url(ws_rpc_url.clone()).await?;

    let config = KeystoreConfig::new().fs_root(keystore_uri.clone());
    let keystore = Keystore::new(config).expect("Failed to create keystore");
    let public = keystore.first_local::<SpSr25519>().unwrap();
    let pair = keystore.get_secret::<SpSr25519>(&public).unwrap();
    let signer = TanglePairSigner::new(pair.0);

    // Get the account ID from the signer for display
    let account_id = signer.account_id();
    println!(
        "{}",
        style(format!(
            "Starting registration process for Operator ID: {}",
            account_id
        ))
        .cyan()
    );

    let ecdsa_public = keystore
        .first_local::<gadget_crypto::sp_core::SpEcdsa>()
        .map_err(|e| color_eyre::eyre::eyre!("Missing ECDSA key: {}", e))?;

    let preferences =
        tangle_subxt::tangle_testnet_runtime::api::services::calls::types::register::Preferences {
            key: decompress_pubkey(&ecdsa_public.0.0).unwrap(),
            price_targets: PriceTargets::default().0,
        };

    info!("Joining operators...");
    let join_call = api::tx()
        .multi_asset_delegation()
        .join_operators(1_000_000_000_000_000);
    let join_res = client
        .tx()
        .sign_and_submit_then_watch_default(&join_call, &signer)
        .await?;

    // Wait for finalization instead of just in-block
    let events = join_res.wait_for_finalized_success().await?;
    info!("Successfully joined operators with events: {:?}", events);

    println!(
        "{}",
        style(format!("Registering for blueprint ID: {}...", blueprint_id)).cyan()
    );
    let registration_args = tangle_subxt::tangle_testnet_runtime::api::services::calls::types::register::RegistrationArgs::new();
    let register_call =
        api::tx()
            .services()
            .register(blueprint_id, preferences, registration_args, 0);
    let register_res = client
        .tx()
        .sign_and_submit_then_watch_default(&register_call, &signer)
        .await?;

    // Wait for finalization instead of just in-block
    let events = register_res.wait_for_finalized_success().await?;
    info!(
        "Successfully registered for blueprint with ID: {} with events: {:?}",
        blueprint_id, events
    );

    // Verify registration by querying the latest block
    println!("{}", style("Verifying registration...").cyan());
    let latest_block = client.blocks().at_latest().await?;
    let latest_block_hash = latest_block.hash();
    info!("Latest block: {:?}", latest_block.number());

    // Create a TangleServicesClient to query operator blueprints
    let services_client =
        gadget_clients::tangle::services::TangleServicesClient::new(client.clone());

    info!("Querying blueprints for account: {:?}", account_id);

    // Query operator blueprints at the latest block
    let block_hash = latest_block_hash.0;
    let blueprints = services_client
        .query_operator_blueprints(block_hash, account_id.clone())
        .await?;

    info!("Found {} blueprints for operator", blueprints.len());
    for (i, blueprint) in blueprints.iter().enumerate() {
        info!("Blueprint {}: {:?}", i, blueprint);
    }

    println!("{}", style("Registration process completed").green());
    Ok(())
}
