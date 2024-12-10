use blueprint_test_utils::test_ext::new_test_ext_blueprint_manager;
use blueprint_test_utils::{
    tangle::NodeConfig, tempfile, wait_for_completion_of_tangle_job, InputValue, Job, OutputValue,
};

const SQUARING_JOB_ID: usize = 0;
const N: usize = 5;

#[gadget_sdk::tokio::test(flavor = "multi_thread", crate = "::gadget_sdk::tokio")]
async fn test_blueprint() {
    blueprint_test_utils::setup_log();

    let tmp_dir = tempfile::TempDir::new().unwrap();
    let tmp_dir_path = tmp_dir.path().to_string_lossy().into_owned();

    new_test_ext_blueprint_manager::<N, 1, String, _, _>(
        tmp_dir_path,
        blueprint_test_utils::run_test_blueprint_manager,
        NodeConfig::new(false),
    )
    .await
    .execute_with_async(|client, handles, blueprint, _| async move {
        let keypair = handles[0].sr25519_id().clone();
        let selected_service = &blueprint.services[0];
        let service_id = selected_service.id;

        gadget_sdk::info!("Submitting job {SQUARING_JOB_ID} with service ID {service_id}");

        let job_args = vec![(InputValue::Uint64(5))];

        let job = blueprint_test_utils::submit_job(
            client,
            &keypair,
            service_id,
            Job::from(SQUARING_JOB_ID as u8),
            job_args,
            0,
        )
        .await
        .expect("Failed to submit job");

        let call_id = job.call_id;

        gadget_sdk::info!(
            "Submitted job {SQUARING_JOB_ID} with service ID {service_id} has call id {call_id}"
        );

        let job_results = wait_for_completion_of_tangle_job(client, service_id, call_id, N)
            .await
            .expect("Failed to wait for job completion");

        assert_eq!(job_results.service_id, service_id);
        assert_eq!(job_results.call_id, call_id);

        let expected_outputs = vec![OutputValue::Uint64(25)];
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
    })
    .await
}
