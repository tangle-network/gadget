use crate::test_ext::NAME_IDS;
use blueprint_manager::config::BlueprintManagerConfig;
use blueprint_manager::executor::BlueprintManagerHandle;
use blueprint_manager::sdk::prelude::color_eyre;
use blueprint_manager::sdk::prelude::tangle_subxt::subxt_signer::ExposeSecret;
use cargo_tangle::deploy::Opts;
use gadget_common::prelude::DebugLogger;
use gadget_common::subxt_signer::{ecdsa, sr25519};
use gadget_common::tangle_runtime::api::services::calls::types::call::{Args, Job};
use gadget_common::tangle_runtime::api::services::calls::types::create_blueprint::Blueprint;
use gadget_common::tangle_runtime::api::services::calls::types::register::{
    Preferences, RegistrationArgs,
};
use gadget_common::tangle_runtime::api::services::storage::types::job_results::JobResults;
use gadget_common::tangle_runtime::{api, AccountId32};
use gadget_common::tangle_subxt::subxt::OnlineClient;
pub use gadget_core::job::SendFuture;
use gadget_io::{GadgetConfig, SupportedChains};
use gadget_sdk::keystore::backend::fs::FilesystemKeystore;
use gadget_sdk::keystore::backend::GenericKeyStore;
use gadget_sdk::keystore::BackendExt;
use libp2p::Multiaddr;
pub use log;
use std::error::Error;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;
use tangle_environment::runtime::TangleConfig;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt::SubscriberBuilder;
use tracing_subscriber::util::SubscriberInitExt;
use url::Url;
use uuid::Uuid;

pub type InputValue = api::runtime_types::tangle_primitives::services::field::Field<AccountId32>;
pub type OutputValue = api::runtime_types::tangle_primitives::services::field::Field<AccountId32>;

pub mod sync;
pub mod test_ext;

pub type TestClient = OnlineClient<TangleConfig>;

pub struct PerTestNodeInput<T> {
    instance_id: u64,
    bind_ip: IpAddr,
    bind_port: u16,
    bootnodes: Vec<Multiaddr>,
    verbose: i32,
    pretty: bool,
    #[allow(dead_code)]
    extra_input: T,
    local_tangle_node: Url,
}

/// Runs a test node using a top-down approach and invoking the blueprint manager to auto manage
/// execution of blueprints and their associated services for the test node.
pub async fn run_test_blueprint_manager<T: Send + Clone + 'static>(
    input: PerTestNodeInput<T>,
) -> BlueprintManagerHandle {
    let tmp_store = Uuid::new_v4().to_string();
    let keystore_uri = PathBuf::from(format!(
        "./target/keystores/{}/{tmp_store}/",
        NAME_IDS[input.instance_id as usize].to_lowercase()
    ));

    let keystore_uri = std::path::absolute(keystore_uri).expect("Failed to resolve keystore URI");

    inject_test_keys(&keystore_uri, input.instance_id as usize)
        .await
        .expect("Failed to inject testing-related SR25519 keys");

    let keystore_uri = keystore_uri
        .canonicalize()
        .expect("Failed to resolve keystore URI");

    let blueprint_manager_config = BlueprintManagerConfig {
        gadget_config: None,
        keystore_uri: format!("file://{}", keystore_uri.display()),
        verbose: input.verbose,
        pretty: input.pretty,
        instance_id: Some(NAME_IDS[input.instance_id as usize].to_string()),
        test_mode: true,
    };

    let gadget_config = GadgetConfig {
        bind_addr: input.bind_ip,
        bind_port: input.bind_port,
        url: input.local_tangle_node,
        bootnodes: input.bootnodes,
        base_path: keystore_uri,
        keystore_password: None,
        chain: SupportedChains::LocalTestnet,
        verbose: input.verbose,
        pretty: input.pretty,
    };

    let shutdown_signal = futures::future::pending();

    match blueprint_manager::run_blueprint_manager(
        blueprint_manager_config,
        gadget_config,
        shutdown_signal,
    )
    .await
    {
        Ok(res) => res,
        Err(err) => {
            log::error!(target: "gadget", "Failed to run blueprint manager: {err}");
            panic!("Failed to run blueprint manager: {err}");
        }
    }
}

async fn inject_test_keys<P: AsRef<Path>>(
    keystore_path: P,
    node_index: usize,
) -> color_eyre::Result<()> {
    let path = keystore_path.as_ref();
    let name = NAME_IDS[node_index];
    tokio::fs::create_dir_all(path).await?;
    // Format: (name, sr public key, sr private key, sr public key, sr private key)
    const TEST_KEYS: [(
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
    ); 5] = [
        (
            "Alice",
            "d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d",
            "e5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a",
            "020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1",
            "cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854",
        ),
        (
            "Bob",
            "8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48",
            "398f0c28f98885e046333d4a41c19cee4c37368a9832c6502f6cfd182e2aef89",
            "0390084fdbf27d2b79d26a4f13f0ccd982cb755a661969143c37cbc49ef5b91f27",
            "79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf",
        ),
        (
            "Charlie",
            "90b5ab205c6974c9ea841be688864633dc9ca8a357843eeacf2314649965fe22",
            "bc1ede780f784bb6991a585e4f6e61522c14e1cae6ad0895fb57b9a205a8f938",
            "0389411795514af1627765eceffcbd002719f031604fadd7d188e2dc585b4e1afb",
            "f8d74108dbe199c4a6e4ef457046db37c325ba3f709b14cabfa1885663e4c589",
        ),
        (
            "Dave",
            "306721211d5404bd9da88e0204360a1a9ab8b87c66c1bc2fcdd37f3c2222cc20",
            "868020ae0687dda7d57565093a69090211449845a7e11453612800b663307246",
            "03bc9d0ca094bd5b8b3225d7651eac5d18c1c04bf8ae8f8b263eebca4e1410ed0c",
            "fa6ba451077fecce7510092e307338e04150ffccc7224c13561a2b079935a5f7",
        ),
        (
            "Eve",
            "e659a7a1628cdd93febc04a4e0646ea20e9f5f0ce097d9a05290d4a9e054df4e",
            "786ad0e2df456fe43dd1f91ebca22e235bc162e0bb8d53c633e8c85b2af68b7a",
            "031d10105e323c4afce225208f71a6441ee327a65b9e646e772500c74d31f669aa",
            "6b30a5e36f608b73e54665c094f97e221554157fcd03e8be7e25ad32f0e1e5b4",
        ),
    ];

    let (_, sr_public_key, sr_private_key, ecdsa_public_key, ecdsa_private_key) = TEST_KEYS
        .iter()
        .find(|(n, _, _, _, _)| *n == name)
        .expect("Invalid test name");

    // Use prefixes to ensure that the keys are designated as sr25519/ecdsa in the filename
    // 0000 - the prefix for sr25519 keys
    // 0002 - the prefix for ecdsa keys
    inject_inner(path, "0000", sr_public_key, sr_private_key).await?;
    inject_inner(path, "0002", ecdsa_public_key, ecdsa_private_key).await?;

    // Sanity checks for testing before proceeding
    // Per the subxt_signer::SecretUri format, we are using the "Parse DEV_PHRASE secret URI with hex phrase and junction" format
    // which is of type 0x{hex_phrase}//{name}
    let sr_suri =
        gadget_common::subxt_signer::SecretUri::from_str(&format!("0x{sr_private_key}//{name}"))
            .expect("Should be valid SURI");
    assert_eq!(
        sr_suri.phrase.expose_secret(),
        &format!("0x{sr_private_key}")
    );

    // For testing purposes, ensure loading the keys work
    let keystore = GenericKeyStore::<parking_lot::RawRwLock>::Fs(
        FilesystemKeystore::open(path).expect("Unable to create Keystore"),
    );

    match keystore.ecdsa_key() {
        Ok(_ecdsa_key) => {
            /*let ecdsa_keypair =
                ecdsa::Keypair::from_uri(&sr_suri).expect("Failed to create ecdsa keypair");
            assert_eq!(ecdsa_keypair.0.secret_bytes(), ecdsa_key.0.secret_bytes());*/
        }
        Err(err) => {
            log::error!(target: "gadget", "Failed to load ecdsa key: {err}");
            panic!("Failed to load ecdsa key: {err}");
        }
    }

    match keystore.sr25519_key() {
        Ok(_sr25519_key) => {}
        Err(err) => {
            log::error!(target: "gadget", "Failed to load sr25519 key: {err}");
            panic!("Failed to load sr25519 key: {err}");
        }
    }

    if name == "Alice" {
        log::error!(target: "gadget", "We know it works for Alice. Remove this to see others fail");
        std::process::exit(1);
    }

    Ok(())
}

async fn inject_inner<P: AsRef<Path>>(
    base_path: P,
    prefix: &str,
    public_key_name: &str,
    private_key_contents: &str,
) -> color_eyre::Result<()> {
    let new_key_path = base_path
        .as_ref()
        .join(format!("{prefix}{public_key_name}"));
    tokio::fs::write(&new_key_path, private_key_contents).await?;
    let decoded = hex::decode(private_key_contents).expect("Failed to decode private key");

    log::info!(target: "gadget", "Successfully wrote private key {prefix} = {private_key_contents} to {} |\nSecret Bytes: {decoded:?}", new_key_path.display());
    Ok(())
}

/// Sets up the default logging as well as setting a panic hook for tests
pub fn setup_log() {
    let _ = SubscriberBuilder::default()
        .with_env_filter(EnvFilter::from_default_env())
        .finish()
        .try_init();

    std::panic::set_hook(Box::new(|info| {
        log::error!(target: "gadget", "Panic occurred: {info:?}");
        std::process::exit(1);
    }));
}

pub async fn create_blueprint(
    client: &TestClient,
    account_id: &sr25519::Keypair,
    blueprint: Blueprint,
) -> Result<(), Box<dyn Error>> {
    let call = api::tx().services().create_blueprint(blueprint);
    let res = client
        .tx()
        .sign_and_submit_then_watch_default(&call, account_id)
        .await?;
    res.wait_for_finalized_success().await?;
    Ok(())
}

pub async fn join_delegators(
    client: &TestClient,
    account_id: &sr25519::Keypair,
) -> Result<(), Box<dyn Error>> {
    let call_pre = api::tx()
        .multi_asset_delegation()
        .join_operators(1_000_000_000_000_000);
    let res_pre = client
        .tx()
        .sign_and_submit_then_watch_default(&call_pre, account_id)
        .await?;
    res_pre.wait_for_finalized_success().await?;
    Ok(())
}

pub async fn register_blueprint(
    client: &TestClient,
    account_id: &sr25519::Keypair,
    blueprint_id: u64,
    preferences: Preferences,
    registration_args: RegistrationArgs,
    logger: &DebugLogger,
) -> Result<(), Box<dyn Error>> {
    logger.info(format!(
        "Registering to blueprint {blueprint_id} to become an operator ..."
    ));
    let call = api::tx()
        .services()
        .register(blueprint_id, preferences, registration_args);
    let res = client
        .tx()
        .sign_and_submit_then_watch_default(&call, account_id)
        .await?;
    res.wait_for_finalized_success().await?;
    Ok(())
}

pub async fn submit_job(
    client: &TestClient,
    user: &sr25519::Keypair,
    service_id: u64,
    job_type: Job,
    job_params: Args,
) -> Result<(), Box<dyn Error>> {
    let call = api::tx().services().call(service_id, job_type, job_params);
    let res = client
        .tx()
        .sign_and_submit_then_watch_default(&call, user)
        .await?;
    let _res = res.wait_for_finalized_success().await?;
    Ok(())
}

/// Registers a service for a given blueprint. This is meant for testing, and will allow any node
/// to make a call to run a service, and will have all nodes running the service.
pub async fn register_service(
    client: &TestClient,
    user: &sr25519::Keypair,
    blueprint_id: u64,
    test_nodes: Vec<AccountId32>,
) -> Result<(), Box<dyn Error>> {
    let call = api::tx().services().request(
        blueprint_id,
        test_nodes.clone(),
        test_nodes,
        1000,
        Default::default(),
    );
    let res = client
        .tx()
        .sign_and_submit_then_watch_default(&call, user)
        .await?;
    res.wait_for_finalized_success().await?;
    Ok(())
}

pub async fn wait_for_completion_of_job(
    client: &TestClient,
    service_id: u64,
    call_id: u64,
) -> Result<JobResults, Box<dyn Error>> {
    loop {
        gadget_io::tokio::time::sleep(Duration::from_millis(100)).await;
        let call = api::storage().services().job_results(service_id, call_id);
        let res = client.storage().at_latest().await?.fetch(&call).await?;

        if let Some(ret) = res {
            return Ok(ret);
        }
    }
}

pub async fn get_next_blueprint_id(client: &TestClient) -> Result<u64, Box<dyn Error>> {
    let call = api::storage().services().next_blueprint_id();
    let res = client
        .storage()
        .at_latest()
        .await?
        .fetch_or_default(&call)
        .await?;
    Ok(res)
}

pub async fn get_next_service_id(client: &TestClient) -> Result<u64, Box<dyn Error>> {
    let call = api::storage().services().next_instance_id();
    let res = client
        .storage()
        .at_latest()
        .await?
        .fetch_or_default(&call)
        .await?;
    Ok(res)
}

pub async fn get_next_call_id(client: &TestClient) -> Result<u64, Box<dyn Error>> {
    let call = api::storage().services().next_job_call_id();
    let res = client
        .storage()
        .at_latest()
        .await?
        .fetch_or_default(&call)
        .await?;
    Ok(res)
}

#[macro_export]
macro_rules! test_blueprint {
    (
        $blueprint_path:expr,
        $blueprint_name:expr,
        $N:expr,
        [$($input:expr),+],
        [$($expected_output:expr),+]
    ) => {
        use $crate::{
            get_next_call_id, get_next_service_id, run_test_blueprint_manager,
            setup_log, submit_job, wait_for_completion_of_job, Opts,
        };

        use $crate::test_ext::new_test_ext_blueprint_manager;

        #[tokio::test(flavor = "multi_thread")]
        async fn test_externalities_standard() {
            setup_log();

            let mut manifest_path = std::env::current_dir().expect("Failed to get current directory");
            manifest_path.push($blueprint_path);
            manifest_path.canonicalize().expect("File could not be normalized");

            let ws_addr = "ws://127.0.0.1:9944";

            let opts = Opts {
                pkg_name: Some($blueprint_name.to_string()),
                rpc_url: ws_addr.to_string(),
                manifest_path,
                signer: None,
                signer_evm: None,
            };

            new_test_ext_blueprint_manager::<$N, 1, (), _, _>(
                (),
                opts,
                run_test_blueprint_manager,
            )
            .await
            .execute_with_async(move |client, handles| async move {
                let keypair = handles[0].sr25519_id().clone();
                let service_id = get_next_service_id(client)
                    .await
                    .expect("Failed to get next service id");
                let call_id = get_next_call_id(client)
                    .await
                    .expect("Failed to get next job id");

                let mut job_args = Args::new();
                for input in [$($input),+] {
                    job_args.push(input);
                }

                submit_job(
                    client,
                    &keypair,
                    service_id,
                    Job::from(call_id as u8),
                    job_args,
                )
                .await
                .expect("Failed to submit job");

                let job_results = wait_for_completion_of_job(client, service_id, call_id)
                    .await
                    .expect("Failed to wait for job completion");

                assert_eq!(job_results.service_id, service_id);
                assert_eq!(job_results.call_id, call_id);

                let expected_outputs = vec![$($expected_output),+];
                assert_eq!(job_results.result.0.len(), expected_outputs.len(), "Number of outputs doesn't match expected");

                for (result, expected) in job_results.result.0.into_iter().zip(expected_outputs.into_iter()) {
                    assert_eq!(result, expected);
                }
            })
            .await
        }
    };
}

#[cfg(test)]
mod test_macros {
    use super::*;

    test_blueprint!(
        "./blueprints/incredible-squaring/Cargo.toml", // Path to the blueprint's toml
        "incredible-squaring-blueprint",               // Name of the package
        5,                                             // Number of nodes
        [InputValue::Uint64(5)],
        [OutputValue::Uint64(25)] // Expected output: each input squared
    );
}

#[cfg(test)]
mod tests_standard {
    use super::*;
    use crate::test_ext::new_test_ext_blueprint_manager;
    use cargo_tangle::deploy::Opts;

    // This test requires that `yarn install` has been executed inside the `./blueprints/incredible-squaring/` directory
    // The other requirement is that there is a locally-running tangle node
    #[tokio::test(flavor = "multi_thread")]
    async fn test_externalities_gadget_starts() {
        setup_log();

        let mut manifest_path = std::env::current_dir().expect("Failed to get current directory");
        manifest_path.push("../blueprints/incredible-squaring/Cargo.toml");
        manifest_path
            .canonicalize()
            .expect("File could not be normalized");

        let opts = Opts {
            pkg_name: Some("incredible-squaring-blueprint".to_string()),
            rpc_url: "ws://127.0.0.1:9944".to_string(),
            manifest_path,
            signer: None,
            signer_evm: None,
        };
        // --ws-external
        const INPUT: u64 = 10;
        const OUTPUT: u64 = INPUT.pow(2);

        new_test_ext_blueprint_manager::<5, 1, (), _, _>((), opts, run_test_blueprint_manager)
            .await
            .execute_with_async(move |client, handles| async move {
                // At this point, blueprint has been deployed, and every node has registered
                // as an operator for the relevant services

                // What's left: Submit a job, wait for the job to finish, then assert the job results
                let keypair = handles[0].sr25519_id().clone();
                // Important! The tests can only run serially, not in parallel, in order to not cause a race condition in IDs
                let service_id = get_next_service_id(client)
                    .await
                    .expect("Failed to get next service id")
                    .saturating_sub(1);
                let call_id = get_next_call_id(client)
                    .await
                    .expect("Failed to get next job id")
                    .saturating_sub(1);

                handles[0].logger().info(format!(
                    "Submitting job with params service ID: {service_id}, call ID: {call_id}"
                ));

                // Pass the arguments
                let mut job_args = Args::new();
                let input =
                    api::runtime_types::tangle_primitives::services::field::Field::Uint64(INPUT);
                job_args.push(input);

                // Next step: submit a job under that service/job id
                if let Err(err) = submit_job(
                    client,
                    &keypair,
                    service_id,
                    Job::from(call_id as u8),
                    job_args,
                )
                .await
                {
                    handles[0]
                        .logger()
                        .error(format!("Failed to submit job: {err}"));
                    panic!("Failed to submit job: {err}");
                }

                // Step 2: wait for the job to complete
                let job_results = wait_for_completion_of_job(client, service_id, call_id)
                    .await
                    .expect("Failed to wait for job completion");

                // Step 3: Get the job results, compare to expected value(s)
                let expected_result =
                    api::runtime_types::tangle_primitives::services::field::Field::Uint64(OUTPUT);
                assert_eq!(job_results.service_id, service_id);
                assert_eq!(job_results.call_id, call_id);
                assert_eq!(job_results.result.0[0], expected_result);
            })
            .await
    }
}
