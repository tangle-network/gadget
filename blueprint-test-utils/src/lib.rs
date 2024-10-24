use crate::test_ext::NAME_IDS;
use api::services::events::JobResultSubmitted;
use blueprint_manager::config::BlueprintManagerConfig;
use blueprint_manager::executor::BlueprintManagerHandle;
use gadget_io::{GadgetConfig, SupportedChains};
use gadget_sdk::clients::tangle::runtime::TangleClient;
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
use alloy_contract::{CallBuilder, CallDecoder};
use alloy_provider::network::Ethereum;
use alloy_provider::Provider;
use alloy_rpc_types_eth::TransactionReceipt;
use alloy_transport::{Transport, TransportResult};
use subxt::tx::Signer;
use subxt::utils::AccountId32;
use url::Url;
use uuid::Uuid;
use gadget_sdk::{info, error};

pub use gadget_sdk::logging::setup_log;

#[allow(unused_imports)]
use cargo_tangle::deploy::Opts;

pub type InputValue = runtime_types::tangle_primitives::services::field::Field<AccountId32>;
pub type OutputValue = runtime_types::tangle_primitives::services::field::Field<AccountId32>;

pub mod anvil;
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
    let name_lower = NAME_IDS[input.instance_id as usize].to_lowercase();

    let tmp_store = Uuid::new_v4().to_string();
    let keystore_uri = PathBuf::from(format!("./target/keystores/{name_lower}/{tmp_store}/",));

    assert!(
        !keystore_uri.exists(),
        "Keystore URI cannot exist: {}",
        keystore_uri.display()
    );

    let data_dir = std::path::absolute(format!("./target/data/{name_lower}"))
        .expect("Failed to get current directory");

    let keystore_uri_normalized =
        std::path::absolute(keystore_uri).expect("Failed to resolve keystore URI");
    let keystore_uri_str = format!("file:{}", keystore_uri_normalized.display());

    inject_test_keys(&keystore_uri_normalized, input.instance_id as usize)
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
        url: input.local_tangle_node,
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

/// Adds keys relevant for the test to the keystore, and performs some necessary
/// cross-compatability tests to ensure key use consistency between different parts of the codebase
pub async fn inject_test_keys<P: AsRef<Path>>(
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
    keystore
        .bls_bn254_generate_from_string(
            "1371012690269088913462269866874713266643928125698382731338806296762673180359922"
                .to_string(),
        )
        .expect("Should be valid BLS seed");

    // Perform sanity checks on conversions between secrets to ensure
    // consistency as the program executes
    let bytes: [u8; 64] = sr.as_ref().secret.to_bytes();
    let secret_key_again = keystore::sr25519::secret_from_bytes(&bytes).expect("Should be valid");
    assert_eq!(&bytes[..], &secret_key_again.to_bytes()[..]);

    let sr2 = TanglePairSigner::new(
        sp_core::sr25519::Pair::from_seed_slice(&bytes).expect("Should be valid SR25519 keypair"),
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
    res.wait_for_finalized_success().await?;
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

    res_pre.wait_for_finalized_success().await?;
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
    res.wait_for_finalized_success().await?;
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
    let _res = res.wait_for_finalized_success().await?;
    Ok(())
}

/// Registers a service for a given blueprint. This is meant for testing, and will allow any node
/// to make a call to run a service, and will have all nodes running the service.
pub async fn register_service(
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
        Default::default(),
        1000,
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
    required_count: usize,
) -> Result<JobResultSubmitted, Box<dyn Error>> {
    let mut count = 0;
    loop {
        let events = client.events().at_latest().await?;
        let results = events.find::<JobResultSubmitted>().collect::<Vec<_>>();
        info!(
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

pub async fn get_receipt<T, P, D>(
    call: CallBuilder<T, P, D, Ethereum>,
) -> TransportResult<TransactionReceipt>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum>,
    D: CallDecoder,
{
    let pending_tx = call.send().await.unwrap();
    let receipt = pending_tx.get_receipt().await?;

    Ok(receipt)
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
            submit_job, wait_for_completion_of_tangle_job, Opts, setup_log,
        };

        use $crate::test_ext::new_test_ext_blueprint_manager;

        #[tokio::test(flavor = "multi_thread")]
        async fn test_externalities_standard() {
            setup_log();
            let mut base_path = std::env::current_dir().expect("Failed to get current directory");

            base_path.push($blueprint_path);
            base_path
                .canonicalize()
                .expect("File could not be normalized");

            let manifest_path = base_path.join("Cargo.toml");


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
    use alloy_primitives::{address, Bytes, U256};
    use alloy_provider::network::EthereumWallet;
    use alloy_provider::{Provider, ProviderBuilder, WsConnect};
    use cargo_tangle::deploy::{Opts, PrivateKeySigner};
    use futures::StreamExt;
    use gadget_sdk::config::Protocol;
    use gadget_sdk::logging::setup_log;
    use gadget_sdk::{error, info};
    use incredible_squaring_aggregator::aggregator::Aggregator;
    use std::str::FromStr;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tokio::time::timeout;
    use IncredibleSquaringTaskManager::TaskResponded;

    const ANVIL_STATE_PATH: &str =
        "./blueprint-test-utils/anvil/deployed_anvil_states/testnet_state.json";

    /// This test requires that `yarn install` has been executed inside the
    /// `./blueprints/incredible-squaring/` directory
    /// The other requirement is that there is a locally-running tangle node

    #[tokio::test(flavor = "multi_thread")]
    #[allow(clippy::needless_return)]
    async fn test_externalities_gadget_starts() {
        setup_log();
        let mut base_path = std::env::current_dir().expect("Failed to get current directory");

        base_path.push("../blueprints/incredible-squaring");
        base_path
            .canonicalize()
            .expect("File could not be normalized");

        let manifest_path = base_path.join("Cargo.toml");

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
                // At this point, blueprint has been deployed, every node has registered
                // as an operator for the relevant services, and, all gadgets are running

                // What's left: Submit a job, wait for the job to finish, then assert the job results
                let keypair = handles[0].sr25519_id().clone();

                let service_id = get_next_service_id(client)
                    .await
                    .expect("Failed to get next service id")
                    .saturating_sub(1);
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

    alloy_sol_types::sol!(
        #[allow(missing_docs)]
        #[sol(rpc)]
        #[derive(Debug)]
        IncredibleSquaringTaskManager,
        "./../blueprints/incredible-squaring-eigenlayer/contracts/out/IncredibleSquaringTaskManager.sol/IncredibleSquaringTaskManager.json"
    );

    alloy_sol_types::sol!(
        #[allow(missing_docs)]
        #[sol(rpc)]
        #[derive(Debug)]
        PauserRegistry,
        "./../blueprints/incredible-squaring-eigenlayer/contracts/out/IPauserRegistry.sol/IPauserRegistry.json"
    );

    alloy_sol_types::sol!(
        #[allow(missing_docs, clippy::too_many_arguments)]
        #[sol(rpc)]
        #[derive(Debug)]
        RegistryCoordinator,
        "./../blueprints/incredible-squaring-eigenlayer/contracts/out/RegistryCoordinator.sol/RegistryCoordinator.json"
    );

    #[tokio::test(flavor = "multi_thread")]
    #[allow(clippy::needless_return)]
    async fn test_eigenlayer_incredible_squaring_blueprint() {
        setup_log();
        let (_container, http_endpoint, ws_endpoint) =
            anvil::start_anvil_container(ANVIL_STATE_PATH, true).await;

        std::env::set_var("EIGENLAYER_HTTP_ENDPOINT", http_endpoint.clone());
        std::env::set_var("EIGENLAYER_WS_ENDPOINT", ws_endpoint.clone());

        // Sleep to give the testnet time to spin up
        tokio::time::sleep(Duration::from_secs(1)).await;

        // Create a provider using the transport
        let provider = alloy_provider::ProviderBuilder::new()
            .with_recommended_fillers()
            .on_http(http_endpoint.parse().unwrap())
            .root()
            .clone()
            .boxed();
        let accounts = provider.get_accounts().await.unwrap();

        // let service_manager_addr = address!("67d269191c92caf3cd7723f116c85e6e9bf55933");
        let registry_coordinator_addr = address!("c3e53f4d16ae77db1c982e75a937b9f60fe63690");
        let operator_state_retriever_addr = address!("1613beb3b2c4f22ee086b2b38c1476a3ce7f78e8");
        // let delegation_manager_addr = address!("dc64a140aa3e981100a9beca4e685f962f0cf6c9");
        // let strategy_manager_addr = address!("5fc8d32690cc91d4c39d9d3abcbd16989f875707");
        let erc20_mock_addr = address!("7969c5ed335650692bc04293b07f5bf2e7a673c0");

        // Deploy the Pauser Registry to the running Testnet
        let pauser_registry = PauserRegistry::deploy(provider.clone()).await.unwrap();
        let &pauser_registry_addr = pauser_registry.address();

        // Create Quorum
        let registry_coordinator =
            RegistryCoordinator::new(registry_coordinator_addr, provider.clone());
        let operator_set_params = RegistryCoordinator::OperatorSetParam {
            maxOperatorCount: 10,
            kickBIPsOfOperatorStake: 100,
            kickBIPsOfTotalStake: 1000,
        };
        let strategy_params = RegistryCoordinator::StrategyParams {
            strategy: erc20_mock_addr,
            multiplier: 1,
        };
        let _receipt = get_receipt(registry_coordinator.createQuorum(
            operator_set_params,
            0,
            vec![strategy_params],
        ))
        .await
        .unwrap();

        // Deploy the Incredible Squaring Task Manager to the running Testnet
        let task_manager_addr = get_receipt(
            super::tests_standard::IncredibleSquaringTaskManager::deploy_builder(
                provider.clone(),
                registry_coordinator_addr,
                10u32,
            ),
        )
        .await
        .unwrap()
        .contract_address
        .unwrap();
        info!("Task Manager: {:?}", task_manager_addr);
        std::env::set_var("TASK_MANAGER_ADDRESS", task_manager_addr.to_string());

        // We create a Task Manager instance for the task spawner
        let task_manager = IncredibleSquaringTaskManager::new(task_manager_addr, provider.clone());
        let task_generator_address = accounts[4];

        // Initialize the Incredible Squaring Task Manager
        let init_receipt = get_receipt(task_manager.initialize(
            pauser_registry_addr,
            accounts[1],
            accounts[9],
            task_generator_address,
        ))
        .await
        .unwrap();
        assert!(init_receipt.status());

        let signer: PrivateKeySigner =
            "0x2a871d0798f97d79848a013d4936a73bf4cc922c825d33c1cf7073dff6d409c6"
                .parse()
                .unwrap();
        let wallet = EthereumWallet::from(signer);
        let (aggregator, _cancellation_token) = Aggregator::new(
            task_manager_addr,
            registry_coordinator_addr,
            operator_state_retriever_addr,
            http_endpoint.clone(),
            ws_endpoint.clone(),
            "127.0.0.1:8081".to_string(),
            wallet,
        )
        .await
        .unwrap();

        // Run the server in a separate thread
        let (handle, aggregator_shutdown_tx) = aggregator.start(ws_endpoint.to_string());

        let successful_responses = Arc::new(Mutex::new(0));
        let successful_responses_clone = successful_responses.clone();

        // Create an event listener for TaskResponded events
        let task_mgr_clone = task_manager.clone();
        let ws_provider = ProviderBuilder::new()
            .on_ws(WsConnect::new(ws_endpoint.clone()))
            .await
            .unwrap()
            .root()
            .clone()
            .boxed();
        let task_response_listener = async move {
            let filter = task_mgr_clone.TaskResponded_filter().filter;
            info!("Filter: {:?}", filter);
            let mut event_stream = match ws_provider.subscribe_logs(&filter).await {
                Ok(stream) => stream.into_stream(),
                Err(e) => {
                    error!("Failed to subscribe to logs: {:?}", e);
                    return;
                }
            };
            info!("Listening for TaskResponded events...");
            while let Some(event) = event_stream.next().await {
                let TaskResponded {
                    taskResponse: _response,
                    ..
                } = event.log_decode::<TaskResponded>().unwrap().inner.data;
                let mut counter = successful_responses.lock().await;
                *counter += 1;
                if *counter >= 1 {
                    break;
                }
            }
        };
        let response_listener_handle = tokio::spawn(task_response_listener);

        // Start the Task Spawner
        let operators = vec![vec![accounts[0]]];
        let quorums = Bytes::from(vec![0]);
        let task_spawner = async move {
            let mut task_count = 0;
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(10000)).await;

                if get_receipt(
                    task_manager
                        .createNewTask(U256::from(2), 100u32, Bytes::from(vec![0]))
                        .from(task_generator_address),
                )
                .await
                .unwrap()
                .status()
                {
                    info!("Deployed a new task");
                    task_count += 1;
                }

                if get_receipt(
                    registry_coordinator
                        .updateOperatorsForQuorum(operators.clone(), quorums.clone()),
                )
                .await
                .unwrap()
                .status()
                {
                    info!("Updated operators for quorum 0");
                }

                tokio::process::Command::new("sh")
                    .arg("-c")
                    .arg(format!(
                        "cast rpc anvil_mine 1 --rpc-url {} > /dev/null",
                        http_endpoint
                    ))
                    .output()
                    .await
                    .unwrap();
                info!("Mined a block...");

                // Break the loop if we've created enough tasks
                if task_count >= 5 {
                    // Create more tasks than we expect responses for
                    break;
                }
            }
        };
        let task_spawner_handle = tokio::spawn(task_spawner);

        info!("Starting Blueprint Binary...");

        let tmp_store = Uuid::new_v4().to_string();
        let keystore_uri = PathBuf::from(format!(
            "./target/keystores/{}/{tmp_store}/",
            NAME_IDS[0].to_lowercase()
        ));
        assert!(
            !keystore_uri.exists(),
            "Keystore URI cannot exist: {}",
            keystore_uri.display()
        );
        let keystore_uri_normalized =
            std::path::absolute(keystore_uri).expect("Failed to resolve keystore URI");
        let keystore_uri_str = format!("file:{}", keystore_uri_normalized.display());

        let mut arguments = vec![];
        arguments.push("run".to_string());

        arguments.extend([
            format!("--bind-addr={}", IpAddr::from_str("127.0.0.1").unwrap()),
            format!("--bind-port={}", 8545u16),
            format!("--url={}", Url::parse("ws://127.0.0.1:8545").unwrap()),
            format!("--keystore-uri={}", keystore_uri_str.clone()),
            format!("--chain={}", SupportedChains::LocalTestnet),
            format!("--verbose={}", 3),
            format!("--pretty={}", true),
            format!("--blueprint-id={}", 0),
            format!("--service-id={}", 0),
            format!("--protocol={}", Protocol::Eigenlayer),
        ]);

        let current_dir = std::env::current_dir().unwrap();
        let program_path = format!(
            "{}/../target/release/incredible-squaring-blueprint-eigenlayer",
            current_dir.display()
        );
        let program_path = PathBuf::from(program_path).canonicalize().unwrap();

        let mut env_vars = vec![
            ("RPC_URL".to_string(), "ws://127.0.0.1:8545".to_string()),
            ("KEYSTORE_URI".to_string(), keystore_uri_str.clone()),
            ("DATA_DIR".to_string(), keystore_uri_str),
            ("BLUEPRINT_ID".to_string(), format!("{}", 0)),
            ("SERVICE_ID".to_string(), format!("{}", 0)),
            ("REGISTRATION_MODE_ON".to_string(), "true".to_string()),
            (
                "OPERATOR_BLS_KEY_PASSWORD".to_string(),
                "BLS_PASSWORD".to_string(),
            ),
            (
                "OPERATOR_ECDSA_KEY_PASSWORD".to_string(),
                "ECDSA_PASSWORD".to_string(),
            ),
        ];

        // Ensure our child process inherits the current processes' environment vars
        env_vars.extend(std::env::vars());

        // Now that the file is loaded, spawn the process
        let mut process_handle = tokio::process::Command::new(program_path.as_os_str())
            .kill_on_drop(true)
            .stdout(std::process::Stdio::inherit()) // Inherit the stdout of this process
            .stderr(std::process::Stdio::inherit()) // Inherit the stderr of this process
            .stdin(std::process::Stdio::null())
            .current_dir(std::env::current_dir().unwrap())
            .envs(env_vars)
            .args(arguments)
            .spawn()
            .unwrap();

        // Wait for the process to complete or timeout
        let timeout_duration = Duration::from_secs(300); // 5 minutes timeout
        let result = timeout(timeout_duration, async {
            loop {
                let count = *successful_responses_clone.lock().await;
                if count >= 1 {
                    return Ok::<(), std::io::Error>(());
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        })
        .await;

        // When you want to shut down the aggregator:
        if let Err(e) = aggregator_shutdown_tx.send(()).await {
            error!("Failed to send shutdown signal to aggregator: {:?}", e);
        } else {
            info!("Sent shutdown signal to aggregator.");
        }

        // Wait for the aggregator to shut down
        if let Err(e) = handle.await {
            error!("Error waiting for aggregator to shut down: {:?}", e);
        }

        // Cancel the task spawner and response listener
        task_spawner_handle.abort();
        response_listener_handle.abort();

        // Check the result
        match result {
            Ok(Ok(())) => {
                info!("Test completed successfully with 3 or more tasks responded to.");
                if let Err(e) = process_handle.kill().await {
                    error!("Failed to kill the process: {:?}", e);
                } else {
                    info!("Process killed successfully.");
                }
            }
            Ok(Err(e)) => {
                error!("Test failed with error: {:?}", e);
                panic!("Test failed");
            }
            Err(_) => {
                error!(
                    "Test timed out after {} seconds",
                    timeout_duration.as_secs()
                );
                panic!("Test timed out");
            }
        }
    }
}
