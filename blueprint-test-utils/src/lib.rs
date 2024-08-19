use blueprint_manager::config::BlueprintManagerConfig;
use blueprint_manager::executor::BlueprintManagerHandle;
#[allow(unused_imports)]
use cargo_tangle::deploy::Opts;
use gadget_common::subxt_signer::sr25519;
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
use libp2p::Multiaddr;
pub use log;
use std::error::Error;
use std::net::IpAddr;
use std::path::PathBuf;
use std::time::Duration;
use tangle_environment::runtime::TangleConfig;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt::SubscriberBuilder;
use tracing_subscriber::util::SubscriberInitExt;
use url::Url;

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
    // Should be in the form: ../tangle/tmp/alice
    base_path: String,
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
    let blueprint_manager_config = BlueprintManagerConfig {
        gadget_config: None,
        keystore_uri: "./target/keystore".to_string(),
        verbose: input.verbose,
        pretty: input.pretty,
        instance_id: Some(format!("Test Node {}", input.instance_id)),
    };

    let gadget_config = GadgetConfig {
        bind_ip: input.bind_ip,
        bind_port: input.bind_port,
        url: input.local_tangle_node,
        bootnodes: input.bootnodes,
        base_path: PathBuf::from(input.base_path),
        keystore_password: None,
        chain: SupportedChains::LocalTestnet,
        verbose: input.verbose,
        pretty: input.pretty,
    };

    let shutdown_signal = futures::future::pending();

    blueprint_manager::run_blueprint_manager(
        blueprint_manager_config,
        gadget_config,
        shutdown_signal,
    )
    .await
    .expect("Failed to run blueprint manager")
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

pub async fn register_blueprint(
    client: &TestClient,
    account_id: &sr25519::Keypair,
    blueprint_id: u64,
    preferences: Preferences,
    registration_args: RegistrationArgs,
) -> Result<(), Box<dyn Error>> {
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
    let res = client.storage().at_latest().await?.fetch(&call).await?;
    if let Some(ret) = res {
        Ok(ret)
    } else {
        Err("Failed to get next blueprint id".into())
    }
}

pub async fn get_next_service_id(client: &TestClient) -> Result<u64, Box<dyn Error>> {
    let call = api::storage().services().next_instance_id();
    let res = client.storage().at_latest().await?.fetch(&call).await?;
    if let Some(ret) = res {
        Ok(ret)
    } else {
        Err("Failed to get next service id".into())
    }
}

pub async fn get_next_call_id(client: &TestClient) -> Result<u64, Box<dyn Error>> {
    let call = api::storage().services().next_job_call_id();
    let res = client.storage().at_latest().await?.fetch(&call).await?;
    if let Some(ret) = res {
        Ok(ret)
    } else {
        Err("Failed to get next job id".into())
    }
}

#[macro_export]
macro_rules! test_blueprint {
    (
        $blueprint_path:expr,
        $blueprint_name:expr,
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

            new_test_ext_blueprint_manager::<1, 1, (), _, _>(
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

        const INPUT: u64 = 10;
        const OUTPUT: u64 = INPUT.pow(2);

        new_test_ext_blueprint_manager::<1, 1, (), _, _>((), opts, run_test_blueprint_manager)
            .await
            .execute_with_async(move |client, handles| async move {
                // At this point, init_blueprint has been deployed, and every node has registered
                // as an operator to the init_blueprint provided

                // What's left: Submit a job, wait for the job to finish, then assert the job results
                let keypair = handles[0].sr25519_id().clone();
                // Important! The tests can only run serially, not in parallel, in order to not cause a race condition in IDs
                let service_id = get_next_service_id(client)
                    .await
                    .expect("Failed to get next service id");
                let call_id = get_next_call_id(client)
                    .await
                    .expect("Failed to get next job id");

                // Pass the argument
                let mut job_args = Args::new();
                let input =
                    api::runtime_types::tangle_primitives::services::field::Field::Uint64(INPUT);
                job_args.push(input);

                // Next step: submit a job under that service/job id
                submit_job(
                    client,
                    &keypair,
                    service_id,
                    Job::from(call_id as u8),
                    job_args,
                )
                .await
                .expect("Failed to submit job");

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
