use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Creates a temporary directory with the incredible-squaring blueprint
///
/// This function creates a temporary directory and populates it with the files needed
/// for the incredible-squaring blueprint. It returns the temporary directory and the path
/// to the blueprint directory.
#[allow(clippy::too_many_lines)]
pub fn create_test_blueprint() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let blueprint_dir = temp_dir.path().join("incredible-squaring");

    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let crates_dir = current_dir
        .parent()
        .expect("Failed to go back one directory");

    // Create directory structure
    fs::create_dir(&blueprint_dir).expect("Failed to create blueprint directory");
    fs::create_dir(blueprint_dir.join("src")).expect("Failed to create src directory");

    // Create contracts directory and subdirectories
    fs::create_dir(blueprint_dir.join("contracts")).expect("Failed to create contracts directory");
    fs::create_dir_all(
        blueprint_dir
            .join("contracts")
            .join("out")
            .join("IncredibleSquaringBlueprint.sol"),
    )
    .expect("Failed to create contract output directories");

    // Create a mock contract JSON file
    fs::write(
        blueprint_dir.join("contracts").join("out").join("IncredibleSquaringBlueprint.sol").join("IncredibleSquaringBlueprint.json"),
        r#"{"abi":[{"type":"function","name":"REWARDS_PALLET","inputs":[],"outputs":[{"name":"","type":"address","internalType":"address"}],"stateMutability":"view"},{"type":"function","name":"ROOT_CHAIN","inputs":[],"outputs":[{"name":"","type":"address","internalType":"address"}],"stateMutability":"view"},{"type":"function","name":"blueprintOwner","inputs":[],"outputs":[{"name":"","type":"address","internalType":"address"}],"stateMutability":"view"},{"type":"function","name":"canJoin","inputs":[{"name":"serviceId","type":"uint64","internalType":"uint64"},{"name":"operator","type":"tuple","internalType":"struct ServiceOperators.OperatorPreferences","components":[{"name":"ecdsaPublicKey","type":"bytes","internalType":"bytes"},{"name":"priceTargets","type":"tuple","internalType":"struct ServiceOperators.PriceTargets","components":[{"name":"cpu","type":"uint64","internalType":"uint64"},{"name":"mem","type":"uint64","internalType":"uint64"},{"name":"storage_hdd","type":"uint64","internalType":"uint64"},{"name":"storage_ssd","type":"uint64","internalType":"uint64"},{"name":"storage_nvme","type":"uint64","internalType":"uint64"}]}]}],"outputs":[{"name":"allowed","type":"bool","internalType":"bool"}],"stateMutability":"view"},{"type":"function","name":"canLeave","inputs":[{"name":"serviceId","type":"uint64","internalType":"uint64"},{"name":"operator","type":"tuple","internalType":"struct ServiceOperators.OperatorPreferences","components":[{"name":"ecdsaPublicKey","type":"bytes","internalType":"bytes"},{"name":"priceTargets","type":"tuple","internalType":"struct ServiceOperators.PriceTargets","components":[{"name":"cpu","type":"uint64","internalType":"uint64"},{"name":"mem","type":"uint64","internalType":"uint64"},{"name":"storage_hdd","type":"uint64","internalType":"uint64"},{"name":"storage_ssd","type":"uint64","internalType":"uint64"},{"name":"storage_nvme","type":"uint64","internalType":"uint64"}]}]}],"outputs":[{"name":"allowed","type":"bool","internalType":"bool"}],"stateMutability":"view"},{"type":"function","name":"currentBlueprintId","inputs":[],"outputs":[{"name":"","type":"uint256","internalType":"uint256"}],"stateMutability":"view"},{"type":"function","name":"masterBlueprintServiceManager","inputs":[],"outputs":[{"name":"","type":"address","internalType":"address"}],"stateMutability":"view"},{"type":"function","name":"masterBlueprintServiceManagerAddress","inputs":[],"outputs":[{"name":"mbsm","type":"address","internalType":"address"}],"stateMutability":"view"},{"type":"function","name":"onApprove","inputs":[{"name":"operator","type":"tuple","internalType":"struct ServiceOperators.OperatorPreferences","components":[{"name":"ecdsaPublicKey","type":"bytes","internalType":"bytes"},{"name":"priceTargets","type":"tuple","internalType":"struct ServiceOperators.PriceTargets","components":[{"name":"cpu","type":"uint64","internalType":"uint64"},{"name":"mem","type":"uint64","internalType":"uint64"},{"name":"storage_hdd","type":"uint64","internalType":"uint64"},{"name":"storage_ssd","type":"uint64","internalType":"uint64"},{"name":"storage_nvme","type":"uint64","internalType":"uint64"}]}]},{"name":"requestId","type":"uint64","internalType":"uint64"},{"name":"restakingPercent","type":"uint8","internalType":"uint8"}],"outputs":[],"stateMutability":"payable"},{"type":"function","name":"onBlueprintCreated","inputs":[{"name":"blueprintId","type":"uint64","internalType":"uint64"},{"name":"owner","type":"address","internalType":"address"},{"name":"mbsm","type":"address","internalType":"address"}],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"onJobCall","inputs":[{"name":"serviceId","type":"uint64","internalType":"uint64"},{"name":"job","type":"uint8","internalType":"uint8"},{"name":"jobCallId","type":"uint64","internalType":"uint64"},{"name":"inputs","type":"bytes","internalType":"bytes"}],"outputs":[],"stateMutability":"payable"},{"type":"function","name":"onJobResult","inputs":[{"name":"serviceId","type":"uint64","internalType":"uint64"},{"name":"job","type":"uint8","internalType":"uint8"},{"name":"jobCallId","type":"uint64","internalType":"uint64"},{"name":"operator","type":"tuple","internalType":"struct ServiceOperators.OperatorPreferences","components":[{"name":"ecdsaPublicKey","type":"bytes","internalType":"bytes"},{"name":"priceTargets","type":"tuple","internalType":"struct ServiceOperators.PriceTargets","components":[{"name":"cpu","type":"uint64","internalType":"uint64"},{"name":"mem","type":"uint64","internalType":"uint64"},{"name":"storage_hdd","type":"uint64","internalType":"uint64"},{"name":"storage_ssd","type":"uint64","internalType":"uint64"},{"name":"storage_nvme","type":"uint64","internalType":"uint64"}]}]},{"name":"inputs","type":"bytes","internalType":"bytes"},{"name":"outputs","type":"bytes","internalType":"bytes"}],"outputs":[],"stateMutability":"payable"},{"type":"function","name":"onOperatorJoined","inputs":[{"name":"serviceId","type":"uint64","internalType":"uint64"},{"name":"operator","type":"tuple","internalType":"struct ServiceOperators.OperatorPreferences","components":[{"name":"ecdsaPublicKey","type":"bytes","internalType":"bytes"},{"name":"priceTargets","type":"tuple","internalType":"struct ServiceOperators.PriceTargets","components":[{"name":"cpu","type":"uint64","internalType":"uint64"},{"name":"mem","type":"uint64","internalType":"uint64"},{"name":"storage_hdd","type":"uint64","internalType":"uint64"},{"name":"storage_ssd","type":"uint64","internalType":"uint64"},{"name":"storage_nvme","type":"uint64","internalType":"uint64"}]}]}],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"onOperatorLeft","inputs":[{"name":"serviceId","type":"uint64","internalType":"uint64"},{"name":"operator","type":"tuple","internalType":"struct ServiceOperators.OperatorPreferences","components":[{"name":"ecdsaPublicKey","type":"bytes","internalType":"bytes"},{"name":"priceTargets","type":"tuple","internalType":"struct ServiceOperators.PriceTargets","components":[{"name":"cpu","type":"uint64","internalType":"uint64"},{"name":"mem","type":"uint64","internalType":"uint64"},{"name":"storage_hdd","type":"uint64","internalType":"uint64"},{"name":"storage_ssd","type":"uint64","internalType":"uint64"},{"name":"storage_nvme","type":"uint64","internalType":"uint64"}]}]}],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"onRegister","inputs":[{"name":"operator","type":"tuple","internalType":"struct ServiceOperators.OperatorPreferences","components":[{"name":"ecdsaPublicKey","type":"bytes","internalType":"bytes"},{"name":"priceTargets","type":"tuple","internalType":"struct ServiceOperators.PriceTargets","components":[{"name":"cpu","type":"uint64","internalType":"uint64"},{"name":"mem","type":"uint64","internalType":"uint64"},{"name":"storage_hdd","type":"uint64","internalType":"uint64"},{"name":"storage_ssd","type":"uint64","internalType":"uint64"},{"name":"storage_nvme","type":"uint64","internalType":"uint64"}]}]},{"name":"registrationInputs","type":"bytes","internalType":"bytes"}],"outputs":[],"stateMutability":"payable"},{"type":"function","name":"onReject","inputs":[{"name":"operator","type":"tuple","internalType":"struct ServiceOperators.OperatorPreferences","components":[{"name":"ecdsaPublicKey","type":"bytes","internalType":"bytes"},{"name":"priceTargets","type":"tuple","internalType":"struct ServiceOperators.PriceTargets","components":[{"name":"cpu","type":"uint64","internalType":"uint64"},{"name":"mem","type":"uint64","internalType":"uint64"},{"name":"storage_hdd","type":"uint64","internalType":"uint64"},{"name":"storage_ssd","type":"uint64","internalType":"uint64"},{"name":"storage_nvme","type":"uint64","internalType":"uint64"}]}]},{"name":"requestId","type":"uint64","internalType":"uint64"}],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"onRequest","inputs":[{"name":"params","type":"tuple","internalType":"struct ServiceOperators.RequestParams","components":[{"name":"requestId","type":"uint64","internalType":"uint64"},{"name":"requester","type":"address","internalType":"address"},{"name":"operators","type":"tuple[]","internalType":"struct ServiceOperators.OperatorPreferences[]","components":[{"name":"ecdsaPublicKey","type":"bytes","internalType":"bytes"},{"name":"priceTargets","type":"tuple","internalType":"struct ServiceOperators.PriceTargets","components":[{"name":"cpu","type":"uint64","internalType":"uint64"},{"name":"mem","type":"uint64","internalType":"uint64"},{"name":"storage_hdd","type":"uint64","internalType":"uint64"},{"name":"storage_ssd","type":"uint64","internalType":"uint64"},{"name":"storage_nvme","type":"uint64","internalType":"uint64"}]}]},{"name":"requestInputs","type":"bytes","internalType":"bytes"},{"name":"permittedCallers","type":"address[]","internalType":"address[]"},{"name":"ttl","type":"uint64","internalType":"uint64"},{"name":"paymentAsset","type":"tuple","internalType":"struct Assets.Asset","components":[{"name":"kind","type":"uint8","internalType":"enum Assets.Kind"},{"name":"data","type":"bytes32","internalType":"bytes32"}]},{"name":"amount","type":"uint256","internalType":"uint256"}]}],"outputs":[],"stateMutability":"nonpayable"}]}"#,
    )
    .expect("Failed to write contract JSON file");

    // Create Cargo.toml
    fs::write(
        blueprint_dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "incredible-squaring-blueprint"
version = "0.1.0"
edition = "2024"

[dependencies]
blueprint-sdk = {{ path = "{}/crates/sdk", features = ["std", "tangle", "macros"] }}
tokio = {{ version = "1.43.0", features = ["rt-multi-thread", "sync", "macros"] }}
tracing-subscriber = {{ version = "0.3.19", features = ["env-filter"] }}
tracing = "0.1.41"
tower = {{ version = "0.5.2", default-features = false }}

[package.metadata.blueprint]
manager = {{ Evm = "ExperimentalBlueprint" }}
master_revision = "Latest"
"#,
            crates_dir.display(),
        ),
    )
    .expect("Failed to write Cargo.toml");

    // Create blueprint.json
    fs::write(
        blueprint_dir.join("blueprint.json"),
        format!(
            r#"{{
  "metadata": {{
    "name": "incredible-squaring-blueprint",
    "description": "A Simple Blueprint to demo how blueprints work on Tangle Network",
    "author": "Tangle Network",
    "category": null,
    "code_repository": "https://github.com/tangle-network/gadget",
    "logo": null,
    "website": "https://tangle.tools",
    "license": "MIT OR Apache-2.0"
  }},
  "manager": {{
    "Evm": "contracts/out/IncredibleSquaringBlueprint.sol/IncredibleSquaringBlueprint.json"
  }},
  "master_manager_revision": "Latest",
  "jobs": [
    {{
      "job_id": 0,
      "metadata": {{
        "name": "xsquare",
        "description": "Returns x^2 saturating to [`u64::MAX`] if overflow occurs."
      }},
      "params": [
        "Uint64"
      ],
      "result": [
        "Uint64"
      ]
    }}
  ],
  "registration_params": [],
  "request_params": [],
  "gadget": {{
    "Native": {{
      "sources": [
      {{
        "fetcher": {{
          "Testing": {{
            "cargo_package": "incredible-squaring-blueprint",
            "cargo_bin": "main",
            "base_path": "{}"
          }}
        }}
      }}
    ]
  }}
  }}
}}"#,
            blueprint_dir.display()
        ),
    )
    .expect("Failed to write blueprint.json");

    // Create main.rs (combined lib.rs and main.rs)
    fs::write(
        blueprint_dir.join("src").join("main.rs"),
        r#"use blueprint_sdk::Job;
use blueprint_sdk::Router;
use blueprint_sdk::contexts::tangle::TangleClientContext;
use blueprint_sdk::crypto::sp_core::SpSr25519;
use blueprint_sdk::crypto::tangle_pair_signer::TanglePairSigner;
use blueprint_sdk::keystore::backends::Backend;
use blueprint_sdk::runner::BackgroundService;
use blueprint_sdk::runner::BlueprintRunner;
use blueprint_sdk::runner::config::BlueprintEnvironment;
use blueprint_sdk::runner::error::RunnerError;
use blueprint_sdk::runner::tangle::config::TangleConfig;
use blueprint_sdk::tangle::consumer::TangleConsumer;
use blueprint_sdk::tangle::extract::{TangleArg, TangleResult};
use blueprint_sdk::tangle::filters::MatchesServiceId;
use blueprint_sdk::tangle::layers::TangleLayer;
use blueprint_sdk::tangle::producer::TangleProducer;
use tokio::sync::oneshot;
use tokio::sync::oneshot::Receiver;
use tower::filter::FilterLayer;
use tracing::error;
use tracing::level_filters::LevelFilter;

// The job ID
pub const XSQUARE_JOB_ID: u32 = 0;

// The job function
pub async fn square(TangleArg(x): TangleArg<u64>) -> TangleResult<u64> {
    let result = x * x;
    
    // The result is then converted into a `JobResult` to be sent back to the caller.
    TangleResult(result)
}

#[derive(Clone)]
pub struct FooBackgroundService;

impl BackgroundService for FooBackgroundService {
    async fn start(&self) -> Result<Receiver<Result<(), RunnerError>>, RunnerError> {
        let (tx, rx) = oneshot::channel();
        tokio::spawn(async move {
            let _ = tx.send(Ok(()));
        });
        Ok(rx)
    }
}

#[tokio::main]
async fn main() -> Result<(), blueprint_sdk::Error> {
    setup_log();

    let env = BlueprintEnvironment::load()?;
    let sr25519_signer = env.keystore().first_local::<SpSr25519>()?;
    let sr25519_pair = env.keystore().get_secret::<SpSr25519>(&sr25519_signer)?;
    let st25519_signer = TanglePairSigner::new(sr25519_pair.0);

    let tangle_client = env.tangle_client().await?;
    let tangle_producer = TangleProducer::finalized_blocks(tangle_client.rpc_client.clone()).await?;
    let tangle_consumer = TangleConsumer::new(tangle_client.rpc_client.clone(), st25519_signer);

    let tangle_config = TangleConfig::default();

    let service_id = env.protocol_settings.tangle()?.service_id.unwrap();
    let result = BlueprintRunner::builder(tangle_config, env)
        .router(
            // A router
            //
            // Each "route" is a job ID and the job function. We can also support arbitrary `Service`s from `tower`,
            // which may make it easier for people to port over existing services to a blueprint.
            Router::new()
                // The route defined here has a `TangleLayer`, which adds metadata to the
                // produced `JobResult`s, making it visible to a `TangleConsumer`.
                .route(XSQUARE_JOB_ID, square.layer(TangleLayer))
                // Add the `FilterLayer` to filter out job calls that don't match the service ID
                .layer(FilterLayer::new(MatchesServiceId(service_id))),
        )
        .background_service(FooBackgroundService)
        // Add potentially many producers
        //
        // A producer is simply a `Stream` that outputs `JobCall`s, which are passed down to the intended
        // job functions.
        .producer(tangle_producer)
        // Add potentially many consumers
        //
        // A consumer is simply a `Sink` that consumes `JobResult`s, which are the output of the job functions.
        // Every result will be passed to every consumer. It is the responsibility of the consumer
        // to determine whether or not to process a result.
        .consumer(tangle_consumer)
        // Custom shutdown handlers
        //
        // Now users can specify what to do when an error occurs and the runner is shutting down.
        // That can be cleanup logic, finalizing database transactions, etc.
        .with_shutdown_handler(async { println!("Shutting down!") })
        .run()
        .await;

    if let Err(e) = result {
        error!("Runner failed! {e:?}");
    }

    Ok(())
}

pub fn setup_log() {
    use tracing_subscriber::util::SubscriberInitExt;

    let _ = tracing_subscriber::fmt::SubscriberBuilder::default()
        .without_time()
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::NONE)
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .finish()
        .try_init();
}"#,
    ).expect("Failed to write main.rs");
    (temp_dir, blueprint_dir)
}
