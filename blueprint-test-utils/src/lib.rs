#![allow(unused_imports)]
use crate::test_ext::{ANVIL_PRIVATE_KEYS, NAME_IDS};
use api::services::events::JobResultSubmitted;
use blueprint_manager::config::BlueprintManagerConfig;
use blueprint_manager::executor::BlueprintManagerHandle;
use gadget_io::{GadgetConfig, SupportedChains};
use gadget_sdk::clients::tangle::runtime::{TangleClient, TangleConfig};
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::sp_arithmetic::per_things::Percent;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::services::calls::types::call::{Args, Job};
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::services::calls::types::create_blueprint::Blueprint;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::services::calls::types::register::{Preferences, RegistrationArgs};
use gadget_sdk::keystore;
use gadget_sdk::keystore::backend::fs::FilesystemKeystore;
use gadget_sdk::keystore::backend::GenericKeyStore;
use gadget_sdk::keystore::{Backend, BackendExt, TanglePairSigner};
use libp2p::Multiaddr;
pub use log;
use sp_core::Pair as PairT;
use std::error::Error;
use std::ffi::OsStr;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;
use alloy_primitives::hex;
use color_eyre::eyre::eyre;
use subxt::tx::{Signer, TxProgress};
use subxt::utils::AccountId32;
use url::Url;
use uuid::Uuid;
use gadget_sdk::{info, error};

pub use gadget_sdk::logging::setup_log;

use cargo_tangle::deploy::Opts;

pub type InputValue = runtime_types::tangle_primitives::services::field::Field<AccountId32>;
pub type OutputValue = runtime_types::tangle_primitives::services::field::Field<AccountId32>;

pub mod anvil;
pub mod binding;
pub mod eigenlayer_test_env;
pub mod helpers;
pub mod symbiotic_test_env;
pub mod sync;
pub mod tangle;
pub mod test_ext;

pub type TestClient = TangleClient;

pub struct PerTestNodeInput<T> {
    instance_id: u64,
    bind_ip: IpAddr,
    bind_port: u16,
    bootnodes: Vec<Multiaddr>,
    verbose: u8,
    pretty: bool,
    #[allow(dead_code)]
    extra_input: T,
    http_rpc_url: Url,
    ws_rpc_url: Url,
}

/// Runs a test node using a top-down approach and invoking the blueprint manager to auto manage
/// execution of blueprints and their associated services for the test node.
pub async fn run_test_blueprint_manager<T: Send + Clone + AsRef<OsStr> + 'static>(
    input: PerTestNodeInput<T>,
) -> BlueprintManagerHandle {
    let name_lower = NAME_IDS[input.instance_id as usize].to_lowercase();

    let keystore_path = Path::new(&input.extra_input);

    let tmp_store = Uuid::new_v4().to_string();
    let keystore_uri = keystore_path.join(format!("keystores/{name_lower}/{tmp_store}/"));

    assert!(
        !keystore_uri.exists(),
        "Keystore URI cannot exist: {}",
        keystore_uri.display()
    );

    let data_dir = std::path::absolute(format!("./target/data/{name_lower}"))
        .expect("Failed to get current directory");

    let keystore_uri_normalized =
        std::path::absolute(keystore_uri.clone()).expect("Failed to resolve keystore URI");
    let keystore_uri_str = format!("file:{}", keystore_uri_normalized.display());

    inject_test_keys(
        &keystore_uri,
        KeyGenType::Tangle(input.instance_id as usize),
    )
    .await
    .expect("Failed to inject testing-related SR25519 keys");

    // Canonicalize to prove the directory exists
    let _ = keystore_uri_normalized
        .canonicalize()
        .expect("Failed to resolve keystore URI");

    let blueprint_manager_config = BlueprintManagerConfig {
        gadget_config: None,
        data_dir,
        keystore_uri: keystore_uri_str.clone(),
        verbose: input.verbose,
        pretty: input.pretty,
        instance_id: Some(NAME_IDS[input.instance_id as usize].to_string()),
        test_mode: true,
    };

    let gadget_config = GadgetConfig {
        bind_addr: input.bind_ip,
        bind_port: input.bind_port,
        http_rpc_url: input.http_rpc_url,
        ws_rpc_url: input.ws_rpc_url,
        bootnodes: input.bootnodes,
        keystore_uri: keystore_uri_str,
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

/// The possible keys to be generated when injecting keys in a keystore for testing.
///
/// - `Random`: A random key will be generated
/// - `Anvil`: Injects one of the premade Anvil key of the given index where that index is 0-9
/// - `Tangle`: Injects the premade Tangle key of the given index where that index is 0-4
///
/// # Errors
///
/// - If the given index is out of bounds for the specified type
/// - Random Key Generation Failure
/// - Generated Key Sanity Check Error
#[derive(Debug, Clone, Copy)]
pub enum KeyGenType {
    Random,
    Anvil(usize),
    Tangle(usize),
}

/// Adds keys for testing to the keystore.
///
/// # Arguments
///
/// - `keystore_path`: The path for the keystore.
/// - `key_gen_type`: The type of key to generate, specified by the [`KeyGenType`] enum.
///
/// # Key Generation
///
/// Depending on the [`KeyGenType`] provided:
/// - `Random`: Generates all random keys.
/// - `Anvil(index)`: Injects the pre-made Anvil ECDSA key from the 10 Anvil dev keys based on the provided index (0-9) and generates a random keys for each other key type
/// - `Tangle(index)`: Injects the pre-made Tangle ED25519, ECDSA, and SR25519 keys based on the provided index (0-4). Randomly generates BLS keys
///
/// # Errors
///
/// This function will return an error if:
/// - The keystore path cannot be created.
/// - Key generation fails for any reason (e.g., invalid seed, random generation failure).
/// - The given index is out of bounds for Anvil or Tangle key types.
///
/// # Returns
///
/// Returns `Ok(())` if the keys were successfully injected, otherwise returns an `Err`.
pub async fn inject_test_keys<P: AsRef<Path>>(
    keystore_path: P,
    key_gen_type: KeyGenType,
) -> color_eyre::Result<()> {
    let path = keystore_path.as_ref();
    tokio::fs::create_dir_all(path).await?;

    match key_gen_type {
        KeyGenType::Random => {
            inject_random_key(&keystore_path)?;
        }
        KeyGenType::Anvil(index) => {
            let private_key = ANVIL_PRIVATE_KEYS[index];
            inject_anvil_key(&keystore_path, private_key)?;
        }
        KeyGenType::Tangle(index) => {
            inject_tangle_key(&keystore_path, NAME_IDS[index])?;
        }
    }

    Ok(())
}

/// Injects the pre-made Anvil key of the given index where that index is 0-9
///
/// # Keys Generated
/// - `SR25519`: Random
/// - `ED25519`: Random
/// - `ECDSA`: Anvil Dev Key
/// - `BLS BN254`: Random
/// - `BLS381`: Random
///
/// # Errors
/// - Fails if the given index is out of bounds
/// - May fail if the keystore path cannot be created or accessed
fn inject_anvil_key<P: AsRef<Path>>(keystore_path: P, seed: &str) -> color_eyre::Result<()> {
    let keystore = GenericKeyStore::<parking_lot::RawRwLock>::Fs(FilesystemKeystore::open(
        keystore_path.as_ref(),
    )?);

    let seed_bytes = hex::decode(&seed[2..]).expect("Invalid hex seed");
    keystore
        .ecdsa_generate_new(Some(&seed_bytes))
        .map_err(|e| eyre!(e))?;
    keystore
        .bls_bn254_generate_new(None)
        .map_err(|e| eyre!(e))?;
    keystore.sr25519_generate_new(None).map_err(|e| eyre!(e))?;
    keystore.ed25519_generate_new(None).map_err(|e| eyre!(e))?;
    keystore.bls381_generate_new(None).map_err(|e| eyre!(e))?;

    Ok(())
}

/// Injects the pre-made Tangle keys of the given index where that index is 0-4
///
/// # Keys Generated
/// - `SR25519`: Tangle Dev Key
/// - `ED25519`: Tangle Dev Key
/// - `ECDSA`: Tangle Dev Key
/// - `BLS BN254`: Random
/// - `BLS381`: Random
///
/// # Indices
/// - 0: Alice
/// - 1: Bob
/// - 2: Charlie
/// - 3:Dave
/// - 4: Eve
///
/// # Errors
/// - Fails if the given index is out of bounds
/// - May fail if the keystore path cannot be created or accessed
fn inject_tangle_key<P: AsRef<Path>>(keystore_path: P, name: &str) -> color_eyre::Result<()> {
    let keystore = GenericKeyStore::<parking_lot::RawRwLock>::Fs(FilesystemKeystore::open(
        keystore_path.as_ref(),
    )?);

    let suri = format!("//{name}");

    let sr = sp_core::sr25519::Pair::from_string(&suri, None).expect("Should be valid SR keypair");
    let sr_seed = &sr.as_ref().secret.to_bytes();

    let ed = sp_core::ed25519::Pair::from_string(&suri, None).expect("Should be valid ED keypair");
    let ed_seed = &ed.seed();

    let ecdsa =
        sp_core::ecdsa::Pair::from_string(&suri, None).expect("Should be valid ECDSA keypair");
    let ecdsa_seed = ecdsa.seed();

    keystore
        .sr25519_generate_new(Some(sr_seed))
        .map_err(|e| eyre!(e))?;
    keystore
        .ed25519_generate_new(Some(ed_seed))
        .map_err(|e| eyre!(e))?;
    keystore
        .ecdsa_generate_new(Some(&ecdsa_seed))
        .map_err(|e| eyre!(e))?;
    keystore.bls381_generate_new(None).map_err(|e| eyre!(e))?;
    keystore
        .bls_bn254_generate_new(None)
        .map_err(|e| eyre!(e))?;

    // Perform sanity checks on conversions between secrets to ensure
    // consistency as the program executes
    let bytes: [u8; 64] = sr.as_ref().secret.to_bytes();
    let secret_key_again =
        keystore::sr25519::secret_from_bytes(&bytes).expect("Invalid SR25519 Bytes");
    assert_eq!(&bytes[..], &secret_key_again.to_bytes()[..]);

    let sr2 = TanglePairSigner::new(
        sp_core::sr25519::Pair::from_seed_slice(&bytes).expect("Invalid SR25519 keypair"),
    );

    let sr1_account_id: AccountId32 = AccountId32(sr.as_ref().public.to_bytes());
    let sr2_account_id: AccountId32 = sr2.account_id().clone();
    assert_eq!(sr1_account_id, sr2_account_id);

    match keystore.ecdsa_key() {
        Ok(ecdsa_key) => {
            assert_eq!(ecdsa_key.signer().seed(), ecdsa_seed);
        }
        Err(err) => {
            log::error!(target: "gadget", "Failed to load ecdsa key: {err}");
            panic!("Failed to load ecdsa key: {err}");
        }
    }

    match keystore.sr25519_key() {
        Ok(sr25519_key) => {
            assert_eq!(sr25519_key.signer().public().0, sr.public().0);
        }
        Err(err) => {
            log::error!(target: "gadget", "Failed to load sr25519 key: {err}");
            panic!("Failed to load sr25519 key: {err}");
        }
    }

    match keystore.ed25519_key() {
        Ok(ed25519_key) => {
            assert_eq!(ed25519_key.signer().public().0, ed.public().0);
        }
        Err(err) => {
            log::error!(target: "gadget", "Failed to load ed25519 key: {err}");
            panic!("Failed to load ed25519 key: {err}");
        }
    }

    Ok(())
}

/// Injects randomly generated keys into the keystore at the given path
///
/// # Keys Generated
/// - `SR25519` - Random
/// - `ED25519` - Random
/// - `ECDSA` - Random
/// - `BLS BN254` - Random
/// - `BLS381` - Random
///
/// # Errors
/// - May fail if the keystore path cannot be created or accessed
pub fn inject_random_key<P: AsRef<Path>>(keystore_path: P) -> color_eyre::Result<()> {
    let keystore = GenericKeyStore::<parking_lot::RawRwLock>::Fs(FilesystemKeystore::open(
        keystore_path.as_ref(),
    )?);
    keystore.sr25519_generate_new(None).map_err(|e| eyre!(e))?;
    keystore.ed25519_generate_new(None).map_err(|e| eyre!(e))?;
    keystore.ecdsa_generate_new(None).map_err(|e| eyre!(e))?;
    keystore
        .bls_bn254_generate_new(None)
        .map_err(|e| eyre!(e))?;
    keystore.bls381_generate_new(None).map_err(|e| eyre!(e))?;
    Ok(())
}

pub async fn create_blueprint(
    client: &TestClient,
    account_id: &TanglePairSigner<sp_core::sr25519::Pair>,
    blueprint: Blueprint,
) -> Result<(), Box<dyn Error>> {
    let call = api::tx().services().create_blueprint(blueprint);
    let res = client
        .tx()
        .sign_and_submit_then_watch_default(&call, account_id)
        .await?;
    wait_for_in_block_success(res).await?;
    Ok(())
}

pub async fn join_delegators(
    client: &TestClient,
    account_id: &TanglePairSigner<sp_core::sr25519::Pair>,
) -> Result<(), Box<dyn Error>> {
    info!("Joining delegators ...");
    let call_pre = api::tx()
        .multi_asset_delegation()
        .join_operators(1_000_000_000_000_000);
    let res_pre = client
        .tx()
        .sign_and_submit_then_watch_default(&call_pre, account_id)
        .await?;

    wait_for_in_block_success(res_pre).await?;
    Ok(())
}

pub async fn register_blueprint(
    client: &TestClient,
    account_id: &TanglePairSigner<sp_core::sr25519::Pair>,
    blueprint_id: u64,
    preferences: Preferences,
    registration_args: RegistrationArgs,
) -> Result<(), Box<dyn Error>> {
    info!("Registering to blueprint {blueprint_id} to become an operator ...");
    let call = api::tx()
        .services()
        .register(blueprint_id, preferences, registration_args);
    let res = client
        .tx()
        .sign_and_submit_then_watch_default(&call, account_id)
        .await?;
    wait_for_in_block_success(res).await?;
    Ok(())
}

pub async fn submit_job(
    client: &TestClient,
    user: &TanglePairSigner<sp_core::sr25519::Pair>,
    service_id: u64,
    job_type: Job,
    job_params: Args,
) -> Result<(), Box<dyn Error>> {
    let call = api::tx().services().call(service_id, job_type, job_params);
    let res = client
        .tx()
        .sign_and_submit_then_watch_default(&call, user)
        .await?;
    wait_for_in_block_success(res).await?;
    Ok(())
}

/// Requests a service with a given blueprint. This is meant for testing, and will allow any node
/// to make a call to run a service, and will have all nodes running the service.
pub async fn request_service(
    client: &TestClient,
    user: &TanglePairSigner<sp_core::sr25519::Pair>,
    blueprint_id: u64,
    test_nodes: Vec<AccountId32>,
) -> Result<(), Box<dyn Error>> {
    let call = api::tx().services().request(
        blueprint_id,
        test_nodes.clone(),
        test_nodes,
        Default::default(),
        vec![0],
        1000,
    );
    let res = client
        .tx()
        .sign_and_submit_then_watch_default(&call, user)
        .await?;
    wait_for_in_block_success(res).await?;
    Ok(())
}

pub async fn wait_for_in_block_success(
    mut res: TxProgress<TangleConfig, TestClient>,
) -> Result<(), Box<dyn Error>> {
    while let Some(Ok(event)) = res.next().await {
        let Some(block) = event.as_in_block() else {
            continue;
        };
        block.wait_for_success().await?;
    }
    Ok(())
}

pub async fn wait_for_completion_of_tangle_job(
    client: &TestClient,
    service_id: u64,
    call_id: u64,
    required_count: usize,
) -> Result<JobResultSubmitted, Box<dyn Error>> {
    let mut count = 0;
    let mut blocks = client.blocks().subscribe_best().await?;
    while let Some(Ok(block)) = blocks.next().await {
        let events = block.events().await?;
        let results = events.find::<JobResultSubmitted>().collect::<Vec<_>>();
        info!(
            %service_id,
            %call_id,
            %required_count,
            %count,
            "Waiting for job completion. Found {} results ...",
            results.len()
        );
        for result in results {
            match result {
                Ok(result) => {
                    if result.service_id == service_id && result.call_id == call_id {
                        count += 1;
                        if count == required_count {
                            return Ok(result);
                        }
                    }
                }
                Err(err) => {
                    error!("Failed to get job result: {err}");
                }
            }
        }
    }
    Err("Failed to get job result".into())
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

/// Approves a service request. This is meant for testing, and will always approve the request.
pub async fn approve_service(
    client: &TestClient,
    caller: &TanglePairSigner<sp_core::sr25519::Pair>,
    request_id: u64,
    restaking_percent: u8,
) -> Result<(), Box<dyn Error>> {
    gadget_sdk::info!("Approving service request ...");
    let call = api::tx()
        .services()
        .approve(request_id, Percent(restaking_percent));
    let res = client
        .tx()
        .sign_and_submit_then_watch_default(&call, caller)
        .await?;
    res.wait_for_finalized_success().await?;
    Ok(())
}

pub async fn get_next_request_id(client: &TestClient) -> Result<u64, Box<dyn Error>> {
    gadget_sdk::info!("Fetching next request ID ...");
    let next_request_id_addr = api::storage().services().next_service_request_id();
    let next_request_id = client
        .storage()
        .at_latest()
        .await
        .expect("Failed to fetch latest block")
        .fetch_or_default(&next_request_id_addr)
        .await
        .expect("Failed to fetch next request ID");
    Ok(next_request_id)
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
            get_next_call_id, run_test_blueprint_manager,
            submit_job, wait_for_completion_of_tangle_job, Opts, setup_log,
        };

        use $crate::test_ext::new_test_ext_blueprint_manager;

        #[tokio::test(flavor = "multi_thread")]
        async fn test_externalities_standard() {
            setup_log();
            let mut base_path = std::env::current_dir().expect("Failed to get current directory");

            let tmp_dir = tempfile::TempDir::new().unwrap(); // Create a temporary directory for the keystores
            let tmp_dir_path = format!("{}", tmp_dir.path().display());

            base_path.push($blueprint_path);
            base_path
                .canonicalize()
                .expect("File could not be normalized");

            let manifest_path = base_path.join("Cargo.toml");


            let http_addr = "http://127.0.0.1:9944";
            let ws_addr = "ws://127.0.0.1:9944";

            let opts = Opts {
                pkg_name: Some($blueprint_name.to_string()),
                http_rpc_url: http_addr.to_string(),
                ws_rpc_url: ws_addr.to_string(),
                manifest_path,
                signer: None,
                signer_evm: None,
            };

            new_test_ext_blueprint_manager::<$N, 1, String, _, _>(
                tmp_dir_path,
                opts,
                run_test_blueprint_manager,
            )
            .await
            .execute_with_async(move |client, handles, blueprint| async move {
                let keypair = handles[0].sr25519_id().clone();
                let selected_service = &blueprint.services[0];
                let service_id = selected_service.id;
                let call_id = get_next_call_id(client)
                    .await
                    .expect("Failed to get next job id");

                info!(
                    "Submitting job with params service ID: {service_id}, call ID: {call_id}"
                );

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

                let job_results = wait_for_completion_of_tangle_job(client, service_id, call_id, $N)
                    .await
                    .expect("Failed to wait for job completion");

                assert_eq!(job_results.service_id, service_id);
                assert_eq!(job_results.call_id, call_id);

                let expected_outputs = vec![$($expected_output),+];
                assert_eq!(job_results.result.len(), expected_outputs.len(), "Number of outputs doesn't match expected");

                for (result, expected) in job_results.result.into_iter().zip(expected_outputs.into_iter()) {
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
        "./blueprints/incredible-squaring-eigen/", // Path to the blueprint's dir
        "incredible-squaring-blueprint",           // Name of the package
        5,                                         // Number of nodes
        [InputValue::Uint64(5)],
        [OutputValue::Uint64(25)] // Expected output: each input squared
    );
}

#[cfg(test)]
mod tests_standard {
    use super::*;
    use crate::test_ext::new_test_ext_blueprint_manager;

    use cargo_tangle::deploy::Opts;
    use gadget_sdk::config::protocol::EigenlayerContractAddresses;
    use gadget_sdk::config::Protocol;
    use gadget_sdk::logging::setup_log;
    use gadget_sdk::{error, info};
    use helpers::BlueprintProcessManager;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    /// This test requires that `yarn install` has been executed inside the
    /// `./blueprints/incredible-squaring/` directory
    /// The other requirement is that there is a locally-running tangle node
    #[tokio::test(flavor = "multi_thread")]
    #[allow(clippy::needless_return)]
    async fn test_externalities_gadget_starts() {
        setup_log();
        let mut base_path = std::env::current_dir().expect("Failed to get current directory");
        let tmp_dir = tempfile::TempDir::new().unwrap(); // Create a temporary directory for the keystores
        let tmp_dir_path = format!("{}", tmp_dir.path().display());

        base_path.push("../blueprints/incredible-squaring");
        base_path
            .canonicalize()
            .expect("File could not be normalized");

        let manifest_path = base_path.join("Cargo.toml");

        let opts = Opts {
            pkg_name: Some("incredible-squaring-blueprint".to_string()),
            http_rpc_url: "http://127.0.0.1:9944".to_string(),
            ws_rpc_url: "ws://127.0.0.1:9944".to_string(),
            manifest_path,
            signer: None,
            signer_evm: None,
        };
        // --ws-external
        const INPUT: u64 = 10;
        const OUTPUT: u64 = INPUT.pow(2);

        new_test_ext_blueprint_manager::<5, 1, String, _, _>(
            tmp_dir_path,
            opts,
            run_test_blueprint_manager,
        )
        .await
        .execute_with_async(move |client, handles, blueprint| async move {
            // At this point, blueprint has been deployed, every node has registered
            // as an operator for the relevant services, and, all gadgets are running

            // What's left: Submit a job, wait for the job to finish, then assert the job results
            let keypair = handles[0].sr25519_id().clone();
            let selected_service = &blueprint.services[0];
            let service_id = selected_service.id;
            let call_id = get_next_call_id(client)
                .await
                .expect("Failed to get next job id")
                .saturating_sub(1);

            info!("Submitting job with params service ID: {service_id}, call ID: {call_id}");

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
                error!("Failed to submit job: {err}");
                panic!("Failed to submit job: {err}");
            }

            // Step 2: wait for the job to complete
            let job_results =
                wait_for_completion_of_tangle_job(client, service_id, call_id, handles.len())
                    .await
                    .expect("Failed to wait for job completion");

            // Step 3: Get the job results, compare to expected value(s)
            let expected_result =
                api::runtime_types::tangle_primitives::services::field::Field::Uint64(OUTPUT);
            assert_eq!(job_results.service_id, service_id);
            assert_eq!(job_results.call_id, call_id);
            assert_eq!(job_results.result[0], expected_result);
        })
        .await
    }
}
