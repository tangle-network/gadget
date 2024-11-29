use crate::tangle::node::{Error, SubstrateNode, TANGLE_NODE_ENV};

pub mod node;
pub mod transactions;

/// Run a Tangle node with the default settings.
/// The node will shut down when the returned handle is dropped.
pub fn run() -> Result<SubstrateNode, Error> {
    let tangle_from_env = std::env::var(TANGLE_NODE_ENV).unwrap_or_else(|_| "tangle".to_string());
    let builder = SubstrateNode::builder()
        .binary_paths([
            "../tangle/target/release/tangle",
            "../../tangle/target/release/tangle",
            &tangle_from_env,
        ])
        .arg("validator")
        .arg_val("rpc-cors", "all")
        .arg_val("rpc-methods", "unsafe")
        .arg("rpc-external")
        .arg_val("sealing", "manual")
        .clone();
    builder.spawn()
}

#[macro_export]
/// A template that makes creating domain-specific macros for tangle-based blueprints easier
macro_rules! tangle_blueprint_test_template {
    (
        $blueprint_path:expr,
        $N:tt,
        $test_logic:expr,
    ) => {
        pub use $crate::{
            run_test_blueprint_manager,
            Opts, setup_log,
            tangle, get_blueprint_base_dir, read_cargo_toml_file,
            submit_job, wait_for_completion_of_tangle_job, Job, Args,
        };

        use $crate::test_ext::new_test_ext_blueprint_manager;

        #[tokio::test(flavor = "multi_thread")]
        async fn test_blueprint() {
            setup_log();
            let tangle_node = tangle::run().expect("Failed to start tangle node");
            let mut base_path = get_blueprint_base_dir();

            let tmp_dir = $crate::tempfile::TempDir::new().unwrap();
            let tmp_dir_path = format!("{}", tmp_dir.path().display());

            base_path.push($blueprint_path);
            base_path
                .canonicalize()
                .expect("File could not be found/normalized");

            let manifest_path = base_path.join("Cargo.toml");
            log::info!(target: "gadget", "Manifest path: {manifest_path:?}");
            let manifest = read_cargo_toml_file(&manifest_path).expect("Failed to read blueprint's Cargo.toml");
            let blueprint_name = manifest.package.as_ref().unwrap().name.clone();

            let ws_port = tangle_node.ws_port();
            let http_rpc_url = format!("http://127.0.0.1:{ws_port}");
            let ws_rpc_url = format!("ws://127.0.0.1:{ws_port}");

            let opts = Opts {
                pkg_name: Some(blueprint_name),
                http_rpc_url,
                ws_rpc_url,
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
            .execute_with_async($test_logic)
            .await
        }
    };
}

#[macro_export]
macro_rules! test_tangle_blueprint {
    (
        $blueprint_path:expr,
        $N:tt,
        $T:tt,
        $job_id:tt,
        [$($inputs:expr),*],
        [$($expected_output:expr),*]
    ) => {
        tangle_blueprint_test_template!(
            $blueprint_path,
            $N,
            |client, handles, blueprint| async move {
                let keypair = handles[0].sr25519_id().clone();
                let selected_service = &blueprint.services[0];
                let service_id = selected_service.id;

                gadget_sdk::info!(
                    "Submitting job {} with service ID {service_id}", $job_id
                );

                let job_args = vec![$($inputs),*];

                let job = submit_job(
                    client,
                    &keypair,
                    service_id,
                    Job::from(0),
                    job_args,
                )
                .await
                .expect("Failed to submit job");

                let call_id = job.call_id;

                gadget_sdk::info!(
                    "Submitted job {} with service ID {service_id} has call id {call_id}", $job_id
                );

                let job_results = wait_for_completion_of_tangle_job(client, service_id, call_id, $T)
                    .await
                    .expect("Failed to wait for job completion");

                assert_eq!(job_results.service_id, service_id);
                assert_eq!(job_results.call_id, call_id);

                let expected_outputs = vec![$($expected_output),*];
                if expected_outputs.is_empty() {
                    gadget_sdk::info!("No expected outputs specified, skipping verification");
                    return
                }

                assert_eq!(job_results.result.len(), expected_outputs.len(), "Number of outputs doesn't match expected");

                for (result, expected) in job_results.result.into_iter().zip(expected_outputs.into_iter()) {
                    assert_eq!(result, expected);
                }
            },
        );
    };
    (
        $blueprint_path:expr,
        $N:tt,
        $job_id:tt,
        [$($input:expr),*],
        [$($expected_output:expr),*]
    ) => {
        test_tangle_blueprint!($blueprint_path, $N, $N, $job_id, [$($input),+], [$($expected_output),+]);
    };
}

#[cfg(test)]
mod test_incredible_squaring {
    use crate::{InputValue, OutputValue};

    const KEYGEN_JOB_ID: usize = 0;
    const N: usize = 5;
    test_tangle_blueprint!(
        "./blueprints/incredible-squaring/", // Path to the blueprint's dir relative to the git repo root, or, if not in a git repo, the current working directory
        N,                                   // Number of nodes
        KEYGEN_JOB_ID,                       // Job ID
        [InputValue::Uint64(5)],             // Inputs
        [OutputValue::Uint64(25)]            // Expected output: input squared
    );
}
