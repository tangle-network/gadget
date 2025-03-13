use std::path::PathBuf;

use dialoguer::console::style;
use gadget_chain_setup::tangle::deploy::{Opts, deploy_to_tangle};
use gadget_chain_setup::tangle::transactions;
use gadget_contexts::tangle::TangleClientContext;
use gadget_crypto::sp_core::{SpEcdsa, SpSr25519};
use gadget_crypto::tangle_pair_signer::TanglePairSigner;
use gadget_keystore::backends::Backend;
use gadget_keystore::{Keystore, KeystoreConfig};
use gadget_testing_utils::tangle::harness::{ENDOWED_TEST_NAMES, generate_env_from_node_id};
use gadget_testing_utils::tangle::keys::inject_tangle_key;
use tangle_subxt::subxt::tx::Signer;
use tempfile::TempDir;
use tokio::fs;
use tokio::signal;
use url::Url;

/// Deploy a blueprint to the Tangle
///
/// # Errors
///
/// Returns a `color_eyre::Report` if an error occurs during deployment.
#[allow(clippy::too_many_lines)]
pub async fn deploy_tangle(
    http_rpc_url: String,
    ws_rpc_url: String,
    package: Option<String>,
    devnet: bool,
    keystore_path: Option<PathBuf>,
    manifest_path: PathBuf,
) -> Result<(), color_eyre::Report> {
    if devnet {
        println!(
            "{}",
            style("Starting local Tangle testnet...").cyan().bold()
        );

        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path().to_path_buf();
        let deploy_dir = temp_path.join("deploy_dir");
        fs::create_dir_all(&deploy_dir).await?;

        // Start Local Tangle Node
        let node = gadget_chain_setup::tangle::run(
            gadget_chain_setup::tangle::NodeConfig::new(false).with_log_target("evm", "trace"),
        )
        .await?;
        let http_endpoint = Url::parse(&format!("http://127.0.0.1:{}", node.ws_port()))?;
        let ws_endpoint = Url::parse(&format!("ws://127.0.0.1:{}", node.ws_port()))?;

        // Create test keystore for Alice (to act as the user)
        let test_keystore_path = PathBuf::from("./test-keystore");
        fs::create_dir_all(&test_keystore_path).await?;
        inject_tangle_key(&test_keystore_path, "//Alice")
            .map_err(|e| color_eyre::Report::msg(format!("Failed to inject Alice's key: {}", e)))?;

        // Create deploy keystore for Bob (for deployment)
        let deploy_keystore_path = PathBuf::from("./deploy-keystore");
        fs::create_dir_all(&deploy_keystore_path).await?;
        inject_tangle_key(&deploy_keystore_path, "//Bob")
            .map_err(|e| color_eyre::Report::msg(format!("Failed to inject Bob's key: {}", e)))?;

        // Set up Alice's environment for MBSM deployment
        let alice_env = generate_env_from_node_id(
            ENDOWED_TEST_NAMES[0],
            http_endpoint.clone(),
            ws_endpoint.clone(),
            deploy_dir.as_path(),
        )
        .await?;

        let alice_client = alice_env.tangle_client().await?;

        println!(
            "{}",
            style("Checking if MBSM needs to be deployed...").cyan()
        );

        // Set up Alice's signers for MBSM deployment
        let alice_keystore_config =
            KeystoreConfig::new().fs_root(test_keystore_path.to_string_lossy().to_string());
        let alice_keystore = Keystore::new(alice_keystore_config)?;

        let alice_sr25519_public = alice_keystore.first_local::<SpSr25519>()?;
        let alice_sr25519_pair = alice_keystore.get_secret::<SpSr25519>(&alice_sr25519_public)?;
        let alice_sr25519_signer = TanglePairSigner::new(alice_sr25519_pair.0);

        let alice_ecdsa_public = alice_keystore.first_local::<SpEcdsa>()?;
        let alice_ecdsa_pair = alice_keystore.get_secret::<SpEcdsa>(&alice_ecdsa_public)?;
        let alice_ecdsa_signer = TanglePairSigner::new(alice_ecdsa_pair.0);
        let alice_alloy_key = alice_ecdsa_signer.alloy_key().map_err(|e| {
            color_eyre::Report::msg(format!("Failed to get Alice's Alloy key: {}", e))
        })?;

        // Set up Bob's signers for blueprint deployment
        let deploy_keystore_uri = deploy_keystore_path.to_string_lossy().to_string();
        let bob_keystore_config = KeystoreConfig::new().fs_root(deploy_keystore_uri.clone());
        let bob_keystore = Keystore::new(bob_keystore_config)?;

        let bob_sr25519_public = bob_keystore.first_local::<SpSr25519>()?;
        let bob_sr25519_pair = bob_keystore.get_secret::<SpSr25519>(&bob_sr25519_public)?;
        let bob_sr25519_signer = TanglePairSigner::new(bob_sr25519_pair.0);

        let bob_ecdsa_public = bob_keystore.first_local::<SpEcdsa>()?;
        let bob_ecdsa_pair = bob_keystore.get_secret::<SpEcdsa>(&bob_ecdsa_public)?;
        let bob_ecdsa_signer = TanglePairSigner::new(bob_ecdsa_pair.0);
        let bob_alloy_key = bob_ecdsa_signer.alloy_key().map_err(|e| {
            color_eyre::Report::msg(format!("Failed to get Bob's Alloy key: {}", e))
        })?;

        // Check if MBSM is already deployed
        let latest_revision = transactions::get_latest_mbsm_revision(&alice_client)
            .await
            .map_err(|e| {
                color_eyre::Report::msg(format!("Failed to get latest MBSM revision: {}", e))
            })?;

        if let Some((rev, addr)) = latest_revision {
            println!(
                "{}",
                style(format!(
                    "MBSM is already deployed at revision #{} at address {}",
                    rev, addr
                ))
                .green()
            );
        } else {
            println!(
                "{}",
                style("MBSM is not deployed, deploying now with Alice's account...").cyan()
            );

            let bytecode = tnt_core_bytecode::bytecode::MASTER_BLUEPRINT_SERVICE_MANAGER;
            transactions::deploy_new_mbsm_revision(
                ws_endpoint.as_str(),
                &alice_client,
                &alice_sr25519_signer,
                alice_alloy_key.clone(),
                bytecode,
                alloy_primitives::address!("0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"),
            )
            .await
            .map_err(|e| color_eyre::Report::msg(format!("Failed to deploy MBSM: {}", e)))?;

            println!("{}", style("MBSM deployed successfully").green());
        }

        println!(
            "{}",
            style(format!(
                "Tangle node running at HTTP: {}, WS: {}",
                http_endpoint, ws_endpoint
            ))
            .green()
        );

        println!(
            "{}",
            style("Deploying blueprint to local Tangle testnet using Bob's account...").cyan()
        );
        let blueprint_id = deploy_to_tangle(Opts {
            pkg_name: package,
            http_rpc_url: http_endpoint.to_string(),
            ws_rpc_url: ws_endpoint.to_string(),
            manifest_path,
            signer: Some(bob_sr25519_signer),
            signer_evm: Some(bob_alloy_key),
        })
        .await?;

        // Get Alice's account ID for display
        let alice_account_id = alice_sr25519_signer.account_id();

        println!("\n{}", style("Local Tangle Testnet Active").green().bold());
        println!(
            "{}",
            style(format!("Blueprint ID: {}", blueprint_id)).green()
        );
        println!(
            "{}",
            style(format!("Alice Account ID: {}", alice_account_id)).green()
        );
        println!(
            "\n{}",
            style("If your blueprint was just generated from the template, you can follow the demo below:").cyan().bold()
        );
        println!(
            "{}",
            style("1. Open a new terminal window, leaving this one running").dim()
        );
        println!(
            "\n{}",
            style("2. Register to your blueprint with Alice's account:").dim()
        );
        println!(
            "   {}",
            style(format!(
                "cargo tangle blueprint register --blueprint-id {} --keystore-uri ./test-keystore",
                blueprint_id
            ))
            .yellow()
        );
        println!("\n{}", style("3. Request service with Bob's account (we will use Alice's account as the target operator):").dim());
        println!("   {}", style(format!("cargo tangle blueprint request-service --blueprint-id {} --target-operators {} --value 0 --keystore-uri ./deploy-keystore", blueprint_id, alice_account_id)).yellow());
        println!("\n{}", style("4. List all service requests:").dim());
        println!(
            "   {}",
            style("cargo tangle blueprint ls".to_string()).yellow()
        );
        println!("\n{}", style("5. Accept the service request (request ID is 0 if you are following this demo, otherwise it will be from the request you want to accept):").dim());
        println!(
            "   {}",
            style("cargo tangle blueprint accept --request-id 0 --keystore-uri ./test-keystore")
                .yellow()
        );
        println!("\n{}", style("6. Run the blueprint:").dim());
        println!(
            "   {}",
            style(
                "cargo tangle blueprint run --protocol tangle --keystore-path ./test-keystore"
                    .to_string()
            )
            .yellow()
        );
        println!(
            "\n{}",
            style("7. Open another terminal window, leaving this one running").dim()
        );
        println!(
            "\n{}",
            style("8. Submit a job for the Running Blueprint to process").dim()
        );
        println!("   {}", style(format!("cargo tangle blueprint submit --job 0 --blueprint-id {} --service-id 0 --keystore-uri ./deploy-keystore", blueprint_id)).yellow());

        println!(
            "\n{}",
            style("The local Tangle testnet will continue running for testing purposes.")
                .cyan()
                .bold()
        );
        println!(
            "{}",
            style("It will remain active so you can interact with your deployed blueprint.").cyan()
        );
        println!(
            "{}",
            style("Press Ctrl+C to stop the testnet when you're done testing...").dim()
        );

        // Wait for Ctrl+C to keep the testnet running
        signal::ctrl_c().await?;
        println!("{}", style("\nShutting down devnet...").yellow());

        // Clean up keystores
        println!("{}", style("Cleaning up keystores...").dim());
        if let Err(e) = fs::remove_dir_all(&test_keystore_path).await {
            println!("Warning: Failed to remove test keystore: {}", e);
        }
        if let Err(e) = fs::remove_dir_all(&deploy_keystore_path).await {
            println!("Warning: Failed to remove deploy keystore: {}", e);
        }
    } else {
        let keystore_path = if let Some(keystore_uri) = keystore_path {
            Some(keystore_uri)
        } else if PathBuf::from("./keystore").exists() {
            Some(PathBuf::from("./keystore"))
        } else {
            None
        };

        let (signer, signer_evm) = if let Some(keystore_uri) = keystore_path {
            let keystore_config = KeystoreConfig::new().fs_root(keystore_uri.clone());
            let keystore = Keystore::new(keystore_config)?;

            let sr25519_public = keystore.first_local::<SpSr25519>()?;
            let sr25519_pair = keystore.get_secret::<SpSr25519>(&sr25519_public)?;
            let sr25519_signer = TanglePairSigner::new(sr25519_pair.0);

            let ecdsa_public = keystore.first_local::<SpEcdsa>()?;
            let ecdsa_pair = keystore.get_secret::<SpEcdsa>(&ecdsa_public)?;
            let ecdsa_signer = TanglePairSigner::new(ecdsa_pair.0);
            let alloy_key = ecdsa_signer
                .alloy_key()
                .map_err(|e| color_eyre::Report::msg(format!("Failed to get Alloy key: {}", e)))?;
            (Some(sr25519_signer), Some(alloy_key))
        } else {
            (None, None)
        };

        let _ = deploy_to_tangle(Opts {
            pkg_name: package,
            http_rpc_url,
            ws_rpc_url,
            manifest_path,
            signer,
            signer_evm,
        })
        .await?;
    }
    Ok(())
}
