use crate::test_ext::NAME_IDS;
use api::services::events::JobResultSubmitted;
use blueprint_manager::config::BlueprintManagerConfig;
use blueprint_manager::executor::BlueprintManagerHandle;
use gadget_io::{GadgetConfig, SupportedChains};
use gadget_sdk::clients::tangle::runtime::{TangleClient};
use gadget_sdk::logger::Logger;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types;
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
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;
use subxt::ext::sp_core::Pair;
use subxt::utils::AccountId32;
use url::Url;
use uuid::Uuid;

#[allow(unused_imports)]
use cargo_tangle::deploy::Opts;

pub type InputValue = runtime_types::tangle_primitives::services::field::Field<AccountId32>;
pub type OutputValue = runtime_types::tangle_primitives::services::field::Field<AccountId32>;

pub mod sync;
pub mod test_ext;

pub type TestClient = TangleClient;

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

    assert!(
        !keystore_uri.exists(),
        "Keystore URI cannot exist: {}",
        keystore_uri.display()
    );

    let keystore_uri = std::path::absolute(keystore_uri).expect("Failed to resolve keystore URI");

    inject_test_keys(&keystore_uri, input.instance_id as usize)
        .await
        .expect("Failed to inject testing-related SR25519 keys");

    let keystore_uri = keystore_uri
        .canonicalize()
        .expect("Failed to resolve keystore URI");

    // TODO: Why is the gadget config here when there is a config below?
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

    let keystore = GenericKeyStore::<parking_lot::RawRwLock>::Fs(FilesystemKeystore::open(
        keystore_path.as_ref(),
    )?);

    let suri = format!("//{name}"); // <---- is the exact same as the ones in the chainspec

    let sr = sp_core::sr25519::Pair::from_string(&suri, None).expect("Should be valid SR keypair");
    // using Pair::from_string is the exact same as TPublic::from_string in the chainspec
    let sr_seed = &sr.as_ref().secret.to_bytes();

    let ecdsa =
        sp_core::ecdsa::Pair::from_string(&suri, None).expect("Should be valid ECDSA keypair");
    // using Pair::from_string is the exact same as TPublic::from_string in the chainspec
    let ecdsa_seed = ecdsa.seed();

    keystore
        .sr25519_generate_new(Some(sr_seed))
        .expect("Should be valid SR25519 seed");
    keystore
        .ecdsa_generate_new(Some(&ecdsa_seed))
        .expect("Should be valid ECDSA seed");

    // Perform sanity checks on conversions between secrets to ensure
    // consistency as the program executes
    let bytes: [u8; 64] = sr.as_ref().secret.to_bytes();
    let secret_key_again = keystore::sr25519::secret_from_bytes(&bytes).expect("Should be valid");
    assert_eq!(&bytes[..], &secret_key_again.to_bytes()[..]);

    use gadget_sdk::tangle_subxt::subxt::ext::sp_core as sp_core2;
    let sr2: TanglePairSigner = TanglePairSigner::new(
        sp_core2::sr25519::Pair::from_seed_slice(&bytes).expect("Should be valid SR25519 keypair"),
    );

    let sr1_account_id: AccountId32 = AccountId32(sr.as_ref().public.to_bytes());
    let sr2_account_id: AccountId32 = sr2.account_id().clone();
    assert_eq!(sr1_account_id, sr2_account_id);

    match keystore.ecdsa_key() {
        Ok(ecdsa_key) => {
            assert_eq!(ecdsa_key.0.secret_bytes(), ecdsa_seed);
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

    Ok(())
}

pub async fn create_blueprint(
    client: &TestClient,
    account_id: &TanglePairSigner,
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
    account_id: &TanglePairSigner,
    logger: &Logger,
) -> Result<(), Box<dyn Error>> {
    logger.info("Joining delegators ...");
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
    account_id: &TanglePairSigner,
    blueprint_id: u64,
    preferences: Preferences,
    registration_args: RegistrationArgs,
    logger: &Logger,
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
    user: &TanglePairSigner,
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
    user: &TanglePairSigner,
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

pub async fn wait_for_completion_of_tangle_job(
    client: &TestClient,
    service_id: u64,
    call_id: u64,
    logger: &Logger,
) -> Result<JobResultSubmitted, Box<dyn Error>> {
    loop {
        let events = client.events().at_latest().await?;
        let results = events.find::<JobResultSubmitted>().collect::<Vec<_>>();
        logger.info(format!(
            "Waiting for job completion. Found {} results ...",
            results.len()
        ));
        for result in results {
            match result {
                Ok(result) => {
                    if result.service_id == service_id && result.call_id == call_id {
                        return Ok(result);
                    }
                }
                Err(err) => {
                    logger.error(format!("Failed to get job result: {err}"));
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(4000)).await;
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
            submit_job, wait_for_completion_of_tangle_job, Opts,
        };

        use $crate::test_ext::new_test_ext_blueprint_manager;

        #[tokio::test(flavor = "multi_thread")]
        async fn test_externalities_standard() {

        let mut base_path = std::env::current_dir().expect("Failed to get current directory");

        base_path.push($blueprint_path);
        base_path
            .canonicalize()
            .expect("File could not be normalized");

        let manifest_path = base_path.join("Cargo.toml");
        let blueprint_test_file = base_path.join("blueprint-test.json");
        let blueprint_file = base_path.join("blueprint.json");

        // cp the blueprint-test-file into the blueprint-file
        tokio::fs::copy(&blueprint_test_file, &blueprint_file)
            .await
            .expect("Failed to copy blueprint-test.json to blueprint.json");

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
            .execute_with_async(move |client, handles, logger| async move {
                let keypair = handles[0].sr25519_id().clone();
                let service_id = get_next_service_id(client)
                    .await
                    .expect("Failed to get next service id");
                let call_id = get_next_call_id(client)
                    .await
                    .expect("Failed to get next job id");

                logger.info(format!(
                    "Submitting job with params service ID: {service_id}, call ID: {call_id}"
                ));

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

                let job_results = wait_for_completion_of_tangle_job(client, service_id, call_id, logger)
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

    /// This test requires that `yarn install` has been executed inside the
    /// `./blueprints/incredible-squaring/` directory
    /// The other requirement is that there is a locally-running tangle node

    #[tokio::test(flavor = "multi_thread")]
    async fn test_externalities_gadget_starts() {
        let mut base_path = std::env::current_dir().expect("Failed to get current directory");

        base_path.push("../blueprints/incredible-squaring");
        base_path
            .canonicalize()
            .expect("File could not be normalized");

        let manifest_path = base_path.join("Cargo.toml");
        let blueprint_test_file = base_path.join("blueprint-test.json");
        let blueprint_file = base_path.join("blueprint.json");

        // cp the blueprint-test-file into the blueprint-file
        tokio::fs::copy(&blueprint_test_file, &blueprint_file)
            .await
            .expect("Failed to copy blueprint-test.json to blueprint.json");

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
            .execute_with_async(move |client, handles, logger| async move {
                // At this point, blueprint has been deployed, every node has registered
                // as an operator for the relevant services, and, all gadgets are running

                // What's left: Submit a job, wait for the job to finish, then assert the job results
                let keypair = handles[0].sr25519_id().clone();

                // TODO: Important! The tests can only run serially, not in parallel, in order to not cause a race condition in IDs
                let service_id = get_next_service_id(client)
                    .await
                    .expect("Failed to get next service id")
                    .saturating_sub(1);
                let call_id = get_next_call_id(client)
                    .await
                    .expect("Failed to get next job id")
                    .saturating_sub(1);

                logger.info(format!(
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
                    logger.error(format!("Failed to submit job: {err}"));
                    panic!("Failed to submit job: {err}");
                }

                // Step 2: wait for the job to complete
                let job_results = wait_for_completion_of_tangle_job(
                    client,
                    service_id,
                    call_id,
                    handles[0].logger(),
                )
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

    #[tokio::test(flavor = "multi_thread")]
    async fn test_eigen_incredible_squaring_blueprint() {
        let mut base_path = std::env::current_dir().expect("Failed to get current directory");

        base_path.push("../blueprints/incredible-squaring");
        base_path
            .canonicalize()
            .expect("File could not be normalized");

        let manifest_path = base_path.join("Cargo.toml");
        let blueprint_test_file = base_path.join("blueprint-test.json");
        let blueprint_file = base_path.join("blueprint.json");

        // cp the blueprint-test-file into the blueprint-file
        tokio::fs::copy(&blueprint_test_file, &blueprint_file)
            .await
            .expect("Failed to copy blueprint-test.json to blueprint.json");

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

        // Start Local Anvil Testnet in the background
        let _ = eigensdk_rs::test_utils::anvil::testnet::incredible_squaring::run_incredible_squaring_testnet().await;

        new_test_ext_blueprint_manager::<5, 1, (), _, _>((), opts, run_test_blueprint_manager)
            .await
            .execute_with_async(move |client, handles, logger| async move {
                // At this point, blueprint has been deployed, every node has registered
                // as an operator for the relevant services, and, all gadgets are running

                // What's left: Submit a job, wait for the job to finish, then assert the job results
                let keypair = handles[0].sr25519_id().clone();

                // TODO: Important! The tests can only run serially, not in parallel, in order to not cause a race condition in IDs
                let service_id = get_next_service_id(client)
                    .await
                    .expect("Failed to get next service id")
                    .saturating_sub(1);
                let call_id = get_next_call_id(client)
                    .await
                    .expect("Failed to get next job id")
                    .saturating_sub(1);

                logger.info(format!(
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
                    logger.error(format!("Failed to submit job: {err}"));
                    panic!("Failed to submit job: {err}");
                }

                // Step 2: wait for the job to complete
                let job_results = wait_for_completion_of_tangle_job(
                    client,
                    service_id,
                    call_id,
                    handles[0].logger(),
                )
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
