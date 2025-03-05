use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Creates a temporary directory with the incredible-squaring blueprint
///
/// This function creates a temporary directory and populates it with the files needed
/// for the incredible-squaring blueprint. It returns the temporary directory and the path
/// to the blueprint directory.
pub fn create_test_blueprint() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let blueprint_dir = temp_dir.path().join("incredible-squaring");

    // Create directory structure
    fs::create_dir(&blueprint_dir).expect("Failed to create blueprint directory");
    fs::create_dir(blueprint_dir.join("src")).expect("Failed to create src directory");

    // Create Cargo.toml
    fs::write(
        blueprint_dir.join("Cargo.toml"),
        r#"[package]
name = "incredible-squaring-blueprint"
version = "0.1.1"
description = "A Simple Blueprint to demo how blueprints work on Tangle Network"
edition = "2021"
license = "MIT OR Apache-2.0"
publish = false

[dependencies]
# Gadget dependencies
blueprint-sdk = { path = "../../../crates/sdk", features = ["std", "macros", "tangle"] }

[dev-dependencies]
blueprint-sdk = { path = "../../../crates/sdk", features = ["std", "tangle", "testing"] }
color-eyre = { version = "0.6", features = ["capture-spantrace", "track-caller"] }
tokio = { version = "1", features = ["full"] }

[build-dependencies]
blueprint-metadata = { path = "../../../crates/blueprint/metadata" }

[package.metadata.blueprint]
manager = { Evm = "IncredibleSquaringBlueprint" }
master_revision = "Latest"
"#,
    )
    .expect("Failed to write Cargo.toml");

    // Create blueprint.json
    fs::write(
        blueprint_dir.join("blueprint.json"),
        r#"{
  "metadata": {
    "name": "incredible-squaring-blueprint",
    "description": "A Simple Blueprint to demo how blueprints work on Tangle Network",
    "author": "Tangle Network",
    "category": null,
    "code_repository": "https://github.com/tangle-network/gadget",
    "logo": null,
    "website": "https://tangle.tools",
    "license": "MIT OR Apache-2.0"
  },
  "manager": {
    "Evm": "contracts/out/IncredibleSquaringBlueprint.sol/IncredibleSquaringBlueprint.json"
  },
  "master_manager_revision": "Latest",
  "jobs": [
    {
      "job_id": 0,
      "metadata": {
        "name": "xsquare",
        "description": "Returns x^2 saturating to [`u64::MAX`] if overflow occurs."
      },
      "params": [
        "Uint64"
      ],
      "result": [
        "Uint64"
      ]
    }
  ],
  "registration_params": [],
  "request_params": [],
  "gadget": {
    "Native": {
      "sources": [
        {
          "fetcher": {
            "Testing": {
              "cargo_package": "incredible-squaring-blueprint",
              "cargo_bin": "main"
            }
          }
        }
      ]
    }
  }
}"#,
    )
    .expect("Failed to write blueprint.json");

    // Create main.rs (combined lib.rs and main.rs)
    fs::write(
        blueprint_dir.join("src").join("main.rs"),
        r#"use std::convert::Infallible;
use blueprint_sdk::event_listeners::tangle::events::TangleEventListener;
use blueprint_sdk::event_listeners::tangle::services::{services_post_processor, services_pre_processor};
use blueprint_sdk::macros::contexts::{ServicesContext, TangleClientContext};
use blueprint_sdk::macros::ext::tangle::tangle_subxt::tangle_testnet_runtime::api::services::events::JobCalled;
use blueprint_sdk::config::GadgetConfiguration;
use blueprint_sdk::logging::info;
use blueprint_sdk::macros::ext::tangle::tangle_subxt::subxt::tx::Signer;
use blueprint_sdk::runners::core::runner::BlueprintRunner;
use blueprint_sdk::runners::tangle::tangle::TangleConfig;

#[derive(Clone, TangleClientContext, ServicesContext)]
pub struct MyContext {
    #[config]
    pub env: GadgetConfiguration,
    #[call_id]
    pub call_id: Option<u64>,
}

#[blueprint_sdk::job(
    id = 0,
    params(x),
    event_listener(
        listener = TangleEventListener<MyContext, JobCalled>,
        pre_processor = services_pre_processor,
        post_processor = services_post_processor,
    ),
)]
/// Returns x^2 saturating to [`u64::MAX`] if overflow occurs.
pub fn xsquare(x: u64, _context: MyContext) -> Result<u64, Infallible> {
    Ok(x.saturating_pow(2))
}

#[blueprint_sdk::main(env)]
async fn main() {
    let context = MyContext {
        env: env.clone(),
        call_id: None,
    };

    let x_square = XsquareEventHandler::new(&env, context).await?;

    info!(
        "Starting the event watcher for {} ...",
        x_square.signer.account_id()
    );

    info!("~~~ Executing the incredible squaring blueprint ~~~");
    let tangle_config = TangleConfig::default();
    BlueprintRunner::new(tangle_config, env)
        .job(x_square)
        .run()
        .await?;

    info!("Exiting...");
    Ok(())
}"#,
    ).expect("Failed to write main.rs");

    // Create build.rs
    fs::write(
        blueprint_dir.join("build.rs"),
        r#"fn main() {
    blueprint_metadata::generate();
}"#,
    )
    .expect("Failed to write build.rs");

    (temp_dir, blueprint_dir)
}

#[tokio::test]
async fn test_create_blueprint() {
    let (temp_dir, blueprint_dir) = create_test_blueprint();

    // Check that the files were created
    assert!(blueprint_dir.join("Cargo.toml").exists());
    assert!(blueprint_dir.join("blueprint.json").exists());
    assert!(blueprint_dir.join("src").join("main.rs").exists());
    assert!(blueprint_dir.join("build.rs").exists());

    // Keep temp_dir in scope to prevent cleanup
    drop(temp_dir);
}

#[tokio::test]
async fn test_run_blueprint() -> Result<(), Box<dyn std::error::Error>> {
    let (temp_dir, blueprint_dir) = create_test_blueprint();

    // Build the blueprint
    let status = std::process::Command::new("cargo")
        .args(["build", "--manifest-path"])
        .arg(blueprint_dir.join("Cargo.toml"))
        .status()?;

    if !status.success() {
        return Err("Failed to build blueprint".into());
    }

    // Keep temp_dir in scope to prevent cleanup
    drop(temp_dir);

    Ok(())
}
