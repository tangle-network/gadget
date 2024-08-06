use crate::test_ext::MockNetwork;
use blueprint_manager::config::BlueprintManagerConfig;
use gadget_common::prelude::{InMemoryBackend, NodeInput};
use gadget_common::tangle_runtime::api;
use gadget_common::tangle_runtime::api::services::storage::types::job_results::JobResults;
use gadget_common::tangle_subxt::subxt::{Config, OnlineClient, SubstrateConfig};
pub use gadget_core::job_manager::SendFuture;
use gadget_io::{GadgetConfig, SupportedChains};
use libp2p::Multiaddr;
pub use log;
use std::error::Error;
use std::net::IpAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use tangle_environment::TangleEnvironment;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt::SubscriberBuilder;
use tracing_subscriber::util::SubscriberInitExt;
use url::Url;

pub mod sync;
pub mod test_ext;

pub type TestClient = OnlineClient<SubstrateConfig>;

pub struct PerTestNodeInput<T> {
    instance_id: u64,
    bind_ip: IpAddr,
    bind_port: u16,
    bootnodes: Vec<Multiaddr>,
    // Should be in the form: ../tangle/tmp/alice
    base_path: String,
    verbose: i32,
    pretty: bool,
    extra_input: T,
    local_tangle_node: Url,
}

/// Runs a test node using a top-down approach and invoking the blueprint manager to auto manage
/// execution of blueprints and their associated services for the test node.
pub async fn run_test_blueprint_manager<T: Send + Clone + 'static>(input: PerTestNodeInput<T>) {
    let blueprint_manager_config = BlueprintManagerConfig {
        gadget_config: None,
        verbose: input.verbose,
        pretty: input.pretty,
        instance_id: Some(format!("Test Node {}", input.instance_id)),
    };

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

    blueprint_manager::run_blueprint_manager(&blueprint_manager_config, gadget_config)
        .await
        .expect("Failed to run blueprint manager");
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

pub async fn wait_for_completion_of_job<T: Config>(
    client: &TestClient,
    service_id: u64,
    job_id: u64,
) -> Result<JobResults, Box<dyn Error>> {
    loop {
        gadget_io::tokio::time::sleep(Duration::from_millis(100)).await;
        let call = api::storage().services().job_results(service_id, job_id);
        let res = client.storage().at_latest().await?.fetch(&call).await?;

        if let Some(ret) = res {
            return Ok(ret);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_ext::new_test_ext_blueprint_manager;

    #[gadget_io::tokio::test(flavor = "multi_thread")]
    async fn test_externalities_gadget_starts() {
        setup_log();
        new_test_ext_blueprint_manager::<1, 1, (), _, _>((), run_test_blueprint_manager)
            .await
            .execute_with_async(|_client| {
                assert_eq!(1, 1);
            })
            .await
    }
}
