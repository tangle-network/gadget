use crate::tangle::node::{Error, SubstrateNode, TANGLE_NODE_ENV};

pub mod node;
pub mod transactions;

/// Run a Tangle node with the default settings.
/// The node will shut down when the returned handle is dropped.
pub fn run() -> Result<SubstrateNode, Error> {
    let tangle_from_env = std::env::var(TANGLE_NODE_ENV).unwrap_or_else(|_| "tangle".to_string());
    let builder = SubstrateNode::builder()
        .binary_paths([
            &tangle_from_env,
            "../tangle/target/release/tangle",
            "../../tangle/target/release/tangle",
            "../../../tangle/target/release/tangle",
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
        $N:tt,
        $test_logic:expr,
    ) => {
        use $crate::test_ext::new_test_ext_blueprint_manager;

        #[::gadget_sdk::tokio::test(flavor = "multi_thread", crate = "::gadget_sdk::tokio")]
        async fn test_blueprint() {
            ::blueprint_test_utils::setup_log();

            let tmp_dir = $crate::tempfile::TempDir::new().unwrap();
            let tmp_dir_path = tmp_dir.path().to_string_lossy().into_owned();

            ::blueprint_test_utils::test_ext::new_test_ext_blueprint_manager::<$N, 1, String, _, _>(
                tmp_dir_path,
                ::blueprint_test_utils::run_test_blueprint_manager,
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
        $N:tt,
        $T:tt,
        $job_id:tt,
        [$($inputs:expr),*],
        [$($expected_output:expr),*],
        $call_id:expr,
    ) => {
        ::blueprint_test_utils::tangle_blueprint_test_template!(
            $N,
            |client, handles, blueprint| async move {
                let keypair = handles[0].sr25519_id().clone();
                let selected_service = &blueprint.services[0];
                let service_id = selected_service.id;

                ::gadget_sdk::info!(
                    "Submitting job {} with service ID {service_id}", $job_id
                );

                let job_args = vec![$($inputs),*];

                let job = ::blueprint_test_utils::submit_job(
                    client,
                    &keypair,
                    service_id,
                    $job_id as ::blueprint_test_utils::Job,
                    job_args,
                    $call_id,
                )
                .await
                .expect("Failed to submit job");

                let call_id = job.call_id;

                ::gadget_sdk::info!(
                    "Submitted job {} with service ID {service_id} has call id {call_id}", $job_id
                );

                let job_results = ::blueprint_test_utils::wait_for_completion_of_tangle_job(client, service_id, call_id, $T)
                    .await
                    .expect("Failed to wait for job completion");

                assert_eq!(job_results.service_id, service_id);
                assert_eq!(job_results.call_id, call_id);

                let expected_outputs = vec![$($expected_output),*];
                if expected_outputs.is_empty() {
                    ::gadget_sdk::info!("No expected outputs specified, skipping verification");
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
        $N:tt,
        $job_id:tt,
        [$($input:expr),*],
        [$($expected_output:expr),*]
        $call_id:expr,
    ) => {
        ::blueprint_test_utils::test_tangle_blueprint!($N, $N, $job_id, [$($input),+], [$($expected_output),+], $call_id);
    };
}
