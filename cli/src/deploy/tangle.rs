use std::path::PathBuf;

use dialoguer::console::style;
use gadget_chain_setup::tangle::deploy::{Opts, deploy_to_tangle};
use gadget_chain_setup::tangle::testnet::SubstrateNode;
use gadget_crypto::sp_core::{SpEcdsa, SpSr25519};
use gadget_crypto::tangle_pair_signer::TanglePairSigner;
use gadget_keystore::backends::Backend;
use gadget_keystore::{Keystore, KeystoreConfig};
use gadget_testing_utils::tangle::keys::inject_tangle_key;
use tempfile::TempDir;
use tokio::fs;
use tokio::signal;
use url::Url;
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

        let mut node_builder = SubstrateNode::builder();

        node_builder
            .arg("--dev")
            .arg("--tmp")
            .arg("--rpc-cors=all")
            .arg("--rpc-methods=unsafe")
            .arg("--rpc-external")
            .arg("--ws-external");

        let node = node_builder
            .spawn()
            .map_err(|e| color_eyre::Report::msg(format!("Failed to start Tangle node: {}", e)))?;

        let ws_port = node.ws_port();

        let http_endpoint = Url::parse(&format!("http://127.0.0.1:{}", ws_port))
            .map_err(|e| color_eyre::Report::msg(format!("Failed to parse HTTP URL: {}", e)))?;
        let ws_endpoint = Url::parse(&format!("ws://127.0.0.1:{}", ws_port))
            .map_err(|e| color_eyre::Report::msg(format!("Failed to parse WS URL: {}", e)))?;

        println!(
            "{}",
            style(format!(
                "Tangle node running at HTTP: {}, WS: {}",
                http_endpoint, ws_endpoint
            ))
            .green()
        );

        let keystore_uri = keystore_path
            .clone()
            .unwrap_or_else(|| PathBuf::from("./test-keystore"))
            .to_string_lossy()
            .to_string();

        let keystore_path_buf = PathBuf::from(&keystore_uri);
        fs::create_dir_all(&keystore_path_buf).await?;

        inject_tangle_key(&keystore_path_buf, "//Alice")
            .map_err(|e| color_eyre::Report::msg(format!("Failed to inject Alice's key: {}", e)))?;

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

        println!(
            "{}",
            style("Deploying blueprint to local Tangle testnet...").cyan()
        );
        let blueprint_id = deploy_to_tangle(Opts {
            pkg_name: package,
            http_rpc_url: http_endpoint.to_string(),
            ws_rpc_url: ws_endpoint.to_string(),
            manifest_path,
            signer: Some(sr25519_signer),
            signer_evm: Some(alloy_key),
        })
        .await?;

        println!("\n{}", style("Local Tangle Testnet Active").green().bold());
        println!(
            "{}",
            style(format!("Blueprint ID: {}", blueprint_id)).green()
        );
        println!(
            "\n{}",
            style("To interact with your blueprint:").cyan().bold()
        );
        println!("{}", style("1. Open a new terminal window").dim());
        println!(
            "{}",
            style("2. Use the following environment variables:").dim()
        );
        println!(
            "   {}",
            style(format!("export WS_RPC_URL={}", ws_endpoint)).yellow()
        );
        println!(
            "   {}",
            style(format!("export HTTP_RPC_URL={}", http_endpoint)).yellow()
        );
        println!(
            "   {}",
            style(format!("export KEYSTORE_URI={}", keystore_uri)).yellow()
        );
        println!("\n{}", style("3. Register your blueprint:").dim());
        println!("   {}", style(format!("cargo tangle blueprint register --ws-rpc-url $WS_RPC_URL --blueprint-id {} --keystore-uri $KEYSTORE_URI", blueprint_id)).yellow());
        println!("\n{}", style("4. Request a service:").dim());
        println!("   {}", style(format!("cargo tangle blueprint request-service --ws-rpc-url $WS_RPC_URL --blueprint-id {} --min-exposure-percent 10 --max-exposure-percent 20 --target-operators <OPERATOR_ID> --value 0 --keystore-uri $KEYSTORE_URI", blueprint_id)).yellow());
        println!("\n{}", style("Press Ctrl+C to stop the testnet...").dim());

        signal::ctrl_c().await?;
        println!("{}", style("\nShutting down devnet...").yellow());
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
