// This file is part of Tangle.
// Copyright (C) 2022-2023 Webb Technologies Inc.
//
// Tangle is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Tangle is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Tangle.  If not, see <http://www.gnu.org/licenses/>.

use crate::PerTestNodeInput;
use futures::StreamExt;
use blueprint_manager::executor::BlueprintManagerHandle;
use blueprint_manager::sdk::entry::SendFuture;
use cargo_tangle::deploy::Opts;
use gadget_sdk::clients::tangle::runtime::TangleClient;
use gadget_sdk::logger::Logger;
use gadget_sdk::tangle_subxt::subxt::OnlineClient;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::{ApprovalPrefrence, PriceTargets};
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::services::calls::types::register::{Preferences, RegistrationArgs};
use libp2p::Multiaddr;
use std::collections::HashSet;
use std::future::Future;
use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration;
use futures::stream::FuturesOrdered;
use url::Url;
use std::path::PathBuf;
use subxt::tx::Signer;
use gadget_sdk::keystore::KeystoreUriSanitizer;
use gadget_sdk::keystore::sp_core_subxt::Pair;

const LOCAL_BIND_ADDR: &str = "127.0.0.1";
const LOCAL_TANGLE_NODE: &str = "ws://127.0.0.1:9944";
pub const NAME_IDS: [&str; 5] = ["Alice", "Bob", "Charlie", "Dave", "Eve"];

/// - `N`: number of nodes
/// - `K`: Number of networks accessible per node (should be equal to the number of services in a given blueprint)
/// - `D`: Any data that you want to pass to pass with NodeInput.
/// - `F`: A function that generates a service's execution via a series of shells. Each shell executes a subset of the service,
///        as each service may have a set of operations that are executed in parallel, sequentially, or concurrently.
#[allow(clippy::async_yields_async)]
pub async fn new_test_ext_blueprint_manager<
    const N: usize,
    const K: usize,
    D: Send + Clone + 'static,
    F: Fn(PerTestNodeInput<D>) -> Fut,
    Fut: SendFuture<'static, BlueprintManagerHandle>,
>(
    additional_params: D,
    mut opts: Opts,
    f: F,
) -> LocalhostTestExt {
    assert!(N > 0, "At least one node is required");
    assert!(N <= NAME_IDS.len(), "Only up to 5 nodes are supported");

    let int_logger = Logger::from("Integration-Test");

    let bind_addrs = (0..N)
        .map(|_| find_open_tcp_bind_port())
        .map(|port| {
            (
                Multiaddr::from_str(&format!("/ip4/{LOCAL_BIND_ADDR}/tcp/{port}"))
                    .expect("Should parse MultiAddr"),
                port,
            )
        })
        .collect::<Vec<_>>();

    // Sanity check: ensure uniqueness
    assert_eq!(
        bind_addrs.iter().map(|r| r.1).collect::<HashSet<_>>().len(),
        bind_addrs.len()
    );

    let multi_addrs = bind_addrs
        .iter()
        .map(|(addr, _)| addr.clone())
        .collect::<Vec<_>>();

    let mut handles = vec![];

    for (node_index, (my_addr, my_port)) in bind_addrs.iter().enumerate() {
        let test_input = PerTestNodeInput {
            instance_id: node_index as _,
            bind_ip: IpAddr::from_str(LOCAL_BIND_ADDR).expect("Should be a valid IP"),
            bind_port: *my_port,
            bootnodes: multi_addrs
                .iter()
                .filter(|addr| *addr != my_addr)
                .cloned()
                .collect(),
            verbose: 4,
            pretty: false,
            extra_input: additional_params.clone(),
            local_tangle_node: Url::parse(&opts.rpc_url).expect("Should parse URL"),
        };

        let handle = f(test_input).await;

        let priv_key = handle
            .ecdsa_id()
            .alloy_key()
            .expect("Should create a private key signer");

        let tg_addr = handle.sr25519_id().account_id();
        let evm_addr = handle.ecdsa_id().account_id();

        int_logger.info(format!("Signer TG address: {tg_addr}"));
        int_logger.info(format!("Signer EVM address: {evm_addr}"));
        int_logger.info(format!("Signer EVM(alloy) address: {}", priv_key.address()));

        if node_index == 0 {
            // Replace the None signer and signer_evm values inside opts with Alice's keys
            opts.signer_evm = Some(priv_key);
            opts.signer = Some(handle.sr25519_id().clone().into_inner());
        }

        handles.push(handle);
    }

    // Step 1: Create the blueprint using alice's identity
    let blueprint_id = match cargo_tangle::deploy::deploy_to_tangle(opts).await {
        Ok(id) => id,
        Err(err) => {
            int_logger.error(format!("Failed to deploy blueprint: {err}"));
            panic!("Failed to deploy blueprint: {err}");
        }
    };

    let client = OnlineClient::from_url(LOCAL_TANGLE_NODE)
        .await
        .expect("Failed to create an account-based localhost client");

    // Step 2: Have each identity register to a blueprint
    let mut futures_ordered = FuturesOrdered::new();
    let registration_args = RegistrationArgs::new();
    // TODO: allow the function called to specify the registration args

    for handle in handles {
        let client = OnlineClient::from_url(LOCAL_TANGLE_NODE)
            .await
            .expect("Failed to create an account-based localhost client");
        let registration_args = registration_args.clone();

        let task = async move {
            let keypair = handle.sr25519_id().clone();
            let logger = handle.logger();
            let key = runtime_types::sp_core::ecdsa::Public(handle.ecdsa_id().signer().public().0);

            let preferences = Preferences {
                key,
                approval: ApprovalPrefrence::None,
                price_targets: PriceTargets {
                    cpu: 0,
                    mem: 0,
                    storage_hdd: 0,
                    storage_ssd: 0,
                    storage_nvme: 0,
                },
            };

            if let Err(err) = super::join_delegators(&client, &keypair, logger).await {
                let err_str = format!("{err}");
                if err_str.contains("MultiAssetDelegation::AlreadyOperator") {
                    handle
                        .logger()
                        .warn(format!("{} is already an operator", keypair.account_id()));
                } else {
                    handle
                        .logger()
                        .error(format!("Failed to join delegators: {err}"));
                    panic!("Failed to join delegators: {err}");
                }
            }

            if let Err(err) = super::register_blueprint(
                &client,
                &keypair,
                blueprint_id,
                preferences,
                registration_args.clone(),
                handle.logger(),
            )
            .await
            {
                handle
                    .logger()
                    .error(format!("Failed to register as operator: {err}"));
                panic!("Failed to register as operator: {err}");
            }

            handle
        };

        futures_ordered.push_back(task);
    }

    let mut handles = futures_ordered
        .collect::<Vec<BlueprintManagerHandle>>()
        .await;

    // Step 3: register a service
    let all_nodes = handles
        .iter()
        .map(|handle| handle.sr25519_id().account_id().clone())
        .collect();

    // Use Alice's account to register the service
    int_logger.info(format!(
        "Registering service for blueprint ID {blueprint_id} using Alice's keys ..."
    ));
    if let Err(err) =
        super::register_service(&client, handles[0].sr25519_id(), blueprint_id, all_nodes).await
    {
        int_logger.error(format!("Failed to register service: {err}"));
        panic!("Failed to register service: {err}");
    }

    // Now, start every blueprint manager. With the blueprint submitted and every operator registered
    // to the blueprint, we can now start the blueprint manager, expecting that the blueprint manager
    // will start the services associated with the blueprint as gadgets.
    for handle in handles.iter_mut() {
        handle.start().expect("Failed to start blueprint manager");
    }

    int_logger.info("Waiting for all nodes to be online ...");
    let all_paths = handles
        .iter()
        .map(|r| r.keystore_uri().to_string())
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    wait_for_test_ready(all_paths, &int_logger).await;
    int_logger.info("All nodes are online");

    LocalhostTestExt {
        client,
        handles,
        logger: int_logger,
    }
}

fn find_open_tcp_bind_port() -> u16 {
    let listener = std::net::TcpListener::bind(format!("{LOCAL_BIND_ADDR}:0"))
        .expect("Should bind to localhost");
    listener
        .local_addr()
        .expect("Should have a local address")
        .port()
}

pub struct LocalhostTestExt {
    client: TangleClient,
    handles: Vec<BlueprintManagerHandle>,
    logger: Logger,
}

impl LocalhostTestExt {
    /// An identity function (For future reverse-compatible changes)
    pub fn execute_with<
        T: FnOnce(&TangleClient, &Vec<BlueprintManagerHandle>, &Logger) -> R + Send + 'static,
        R: Send + 'static,
    >(
        &self,
        function: T,
    ) -> R {
        function(&self.client, &self.handles, &self.logger)
    }

    /// An identity function (For future reverse-compatible changes)
    pub async fn execute_with_async<
        'a,
        'b: 'a,
        T: FnOnce(&'a TangleClient, &'a Vec<BlueprintManagerHandle>, &'a Logger) -> R + Send + 'a,
        R: Future<Output = Out> + Send + 'a,
        Out: Send + 'b,
    >(
        &'a self,
        function: T,
    ) -> Out {
        function(&self.client, &self.handles, &self.logger).await
    }
}

/// `base_paths`: All the paths pointing to the keystore for each node
/// This function returns when every test_started.tmp file exists
async fn wait_for_test_ready(base_paths: Vec<PathBuf>, logger: &Logger) {
    let paths = base_paths
        .into_iter()
        .map(|r| r.join("test_started.tmp"))
        .map(|r| r.sanitize_file_path())
        .collect::<Vec<_>>();
    logger.info(format!("Waiting for these paths to exist: {paths:?}"));
    loop {
        let mut ready_count = 0;
        for path in &paths {
            if path.exists() {
                ready_count += 1;
            }
        }

        if ready_count == paths.len() {
            break;
        }

        logger.info(format!(
            "Not all operators are ready yet ({ready_count}/{}). Waiting ...",
            paths.len()
        ));
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}
