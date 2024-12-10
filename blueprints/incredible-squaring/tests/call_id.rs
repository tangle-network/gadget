use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

pub use blueprint_test_utils::{
    get_blueprint_base_dir, read_cargo_toml_file, run_test_blueprint_manager, setup_log,
    submit_job, tangle, wait_for_completion_of_tangle_job, Args, Job, Opts,
};

use blueprint_test_utils::test_ext::new_test_ext_blueprint_manager;
use blueprint_test_utils::{InputValue, OutputValue};

#[gadget_sdk::tokio::test(crate = "::gadget_sdk::tokio")]
async fn call_id_valid() {
    const ITERATIONS: u8 = 5;
    static CALL_ID: AtomicU64 = AtomicU64::new(0);

    setup_log();

    const N: usize = 1;
    const JOB_ID: usize = 1;

    new_test_ext_blueprint_manager::<N, 1, String, _, _>(
        String::new(),
        run_test_blueprint_manager,
        false,
    )
    .await
    .execute_with_async(|client, handles, blueprint, _| async move {
        let keypair = handles[0].sr25519_id().clone();
        let selected_service = &blueprint.services[0];
        let service_id = selected_service.id;

        for _ in 0..ITERATIONS {
            gadget_sdk::info!("Submitting job {JOB_ID} with service ID {service_id}");

            let expected_call_id = CALL_ID.load(Ordering::Relaxed);
            let job_args = vec![InputValue::Uint64(expected_call_id)];

            let job = submit_job(
                client,
                &keypair,
                service_id,
                Job::from(JOB_ID as u8),
                job_args,
                expected_call_id,
            )
            .await
            .expect("Failed to submit job");

            let call_id = job.call_id;

            gadget_sdk::info!(
                "Submitted job {JOB_ID} with service ID {service_id} has call id {call_id}"
            );

            let job_results = wait_for_completion_of_tangle_job(client, service_id, call_id, N)
                .await
                .expect("Failed to wait for job completion");

            assert_eq!(job_results.service_id, service_id);
            assert_eq!(job_results.call_id, call_id);

            let expected_outputs = vec![OutputValue::Uint64(0)];
            if expected_outputs.is_empty() {
                gadget_sdk::info!("No expected outputs specified, skipping verification");
                return;
            }

            assert_eq!(
                job_results.result.len(),
                expected_outputs.len(),
                "Number of outputs doesn't match expected"
            );

            for (result, expected) in job_results
                .result
                .into_iter()
                .zip(expected_outputs.into_iter())
            {
                assert_eq!(result, expected);
            }

            gadget_sdk::info!(
                "*** Finished iteration {} ***",
                CALL_ID.load(Ordering::Relaxed)
            );
            CALL_ID.fetch_add(1, Ordering::Relaxed);
        }
    })
    .await
}
