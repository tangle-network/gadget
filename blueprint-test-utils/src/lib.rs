use blueprint_manager::config::BlueprintManagerConfig;
use blueprint_manager::executor::BlueprintManagerHandle;
use gadget_common::subxt_signer::sr25519;
use gadget_common::tangle_runtime::api;
use gadget_common::tangle_runtime::api::services::calls::types::call::{Args, Job};
use gadget_common::tangle_runtime::api::services::calls::types::create_blueprint::Blueprint;
use gadget_common::tangle_runtime::api::services::calls::types::register::{
    Preferences, RegistrationArgs,
};
use gadget_common::tangle_runtime::api::services::storage::types::job_results::JobResults;
use gadget_common::tangle_subxt::subxt::OnlineClient;
pub use gadget_core::job_manager::SendFuture;
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
        verbose: input.verbose,
        pretty: input.pretty,
        instance_id: Some(format!("Test Node {}", input.instance_id)),
    };

    // The same format as the node_key in ./shell-configs/local-testnet-{input.instance_id}.toml
    let node_key = format!(
        "000000000000000000000000000000000000000000000000000000000000000{}",
        input.instance_id
    );

    let gadget_config = GadgetConfig {
        bind_ip: input.bind_ip,
        bind_port: input.bind_port,
        url: input.local_tangle_node,
        bootnodes: input.bootnodes,
        node_key: Some(node_key),
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
    job_id: u64,
) -> Result<JobResults, Box<dyn Error>> {
    loop {
        gadget_io::tokio::time::sleep(Duration::from_millis(100)).await;
        // Most of what we need is here: api::tx().services() [..] .create_blueprint()
        let call = api::storage().services().job_results(service_id, job_id);
        let res = client.storage().at_latest().await?.fetch(&call).await?;
        // client.tx() to send something to the chain

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

pub async fn get_next_job_id(client: &TestClient) -> Result<u64, Box<dyn Error>> {
    let call = api::storage().services().next_job_call_id();
    let res = client.storage().at_latest().await?.fetch(&call).await?;
    if let Some(ret) = res {
        Ok(ret)
    } else {
        Err("Failed to get next job id".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_ext::new_test_ext_blueprint_manager;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_externalities_gadget_starts() {
        setup_log();

        let init_blueprint = cargo_gadget::deploy::generate_service_blueprint(
            "../../blueprints/incredible-squaring/",
            Some(&"incredible-squaring-blueprint".to_string()),
            "ws://127.0.0.1:9944",
        )
        .await
        .expect("Service blueprint generation failed");

        const INPUT: u64 = 10;
        const OUTPUT: u64 = INPUT.pow(2);

        new_test_ext_blueprint_manager::<1, 1, (), _, _>(
            (),
            init_blueprint.clone(),
            run_test_blueprint_manager,
        )
        .await
        .execute_with_async(|client, handles| async move {
            // At this point, init_blueprint has been deployed, and, every node has registered
            // as an operator to the init_blueprint provided

            // What's left: Submit a job, wait for the job to finish, then assert the job results
            let keypair = handles[0].sr25519_id().clone();
            // Important! The tests can only run serially, not in parallel, in order to not cause a race condition in IDs
            let service_id = get_next_service_id(client)
                .await
                .expect("Failed to get next service id");
            let job_id = get_next_job_id(client)
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
                Job::from(job_id as u8),
                job_args,
            )
            .await
            .expect("Failed to submit job");

            // Step 2: wait for the job to complete
            let job_results = wait_for_completion_of_job(client, service_id, job_id)
                .await
                .expect("Failed to wait for job completion");

            // Step 3: Get the job results, compare to expected value(s)
            let expected_result =
                api::runtime_types::tangle_primitives::services::field::Field::Uint64(OUTPUT);
            assert_eq!(job_results.service_id, service_id);
            assert_eq!(job_results.call_id, job_id);
            assert_eq!(job_results.result.0[0], expected_result);
        })
        .await
    }
}