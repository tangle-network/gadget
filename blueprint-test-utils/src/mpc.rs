#[macro_export]
macro_rules! mpc_generate_keygen_and_signing_tests {
    (
        $blueprint_path:literal,
        $N:tt,
        $T:tt,
        $keygen_job_id:tt,
        [$($keygen_inputs:expr),*],
        [$($expected_keygen_outputs:expr),*],
        $signing_job_id:tt,
        [$($signing_inputs:expr),*],
        [$($expected_signing_outputs:expr),*],
        $atomic_keygen_call_id_store:expr,
    ) => {
        $crate::tangle_blueprint_test_template!(
            $blueprint_path,
            $N,
            |client, handles, blueprint| async move {
                let keypair = handles[0].sr25519_id().clone();
                let service = &blueprint.services[$keygen_job_id as usize];

                let service_id = service.id;
                gadget_sdk::info!(
                    "Submitting KEYGEN job {} with service ID {service_id}", $keygen_job_id
                );

                let job_args = vec![$($keygen_inputs),*];

                let job = submit_job(
                    client,
                    &keypair,
                    service_id,
                    Job::from(0),
                    job_args,
                )
                .await
                .expect("Failed to submit job");

                let keygen_call_id = job.call_id;
                $atomic_keygen_call_id_store.store(keygen_call_id, std::sync::atomic::Ordering::Relaxed);

                gadget_sdk::info!(
                    "Submitted KEYGEN job {} with service ID {service_id} has call id {keygen_call_id}", $keygen_job_id,
                );

                let job_results = wait_for_completion_of_tangle_job(client, service_id, keygen_call_id, $T)
                    .await
                    .expect("Failed to wait for job completion");

                assert_eq!(job_results.service_id, service_id);
                assert_eq!(job_results.call_id, keygen_call_id);

                let expected_outputs = vec![$($expected_keygen_outputs),*];
                if !expected_outputs.is_empty() {
                    assert_eq!(job_results.result.len(), expected_outputs.len(), "Number of keygen outputs doesn't match expected");

                    for (result, expected) in job_results.result.into_iter().zip(expected_outputs.into_iter()) {
                        assert_eq!(result, expected);
                    }
                } else {
                    gadget_sdk::info!("No expected outputs specified, skipping keygen verification");
                }

                log::info!("Keygen job completed successfully! Moving on to signing ...");

                // ~~~~~ Now, run a signing job ~~~~~
                let service = &blueprint.services[0];

                let service_id = service.id;
                gadget_sdk::info!(
                    "Submitting SIGNING job {} with service ID {service_id}", $signing_job_id
                );

                // Pass the arguments
                let job_args = vec![$($signing_inputs),*];

                 let job = submit_job(
                    client,
                    &keypair,
                    service_id,
                    Job::from(0),
                    job_args,
                )
                .await
                .expect("Failed to submit job");

                let signing_call_id = job.call_id;

                gadget_sdk::info!(
                    "Submitted SIGNING job {} with service ID {service_id} has call id {signing_call_id}", $signing_job_id
                );

                let job_results =
                    wait_for_completion_of_tangle_job(client, service_id, signing_call_id, $T)
                        .await
                        .expect("Failed to wait for job completion");

                let expected_outputs = vec![$($expected_signing_outputs),*];
                if !expected_outputs.is_empty() {
                    assert_eq!(job_results.result.len(), expected_outputs.len(), "Number of signing outputs doesn't match expected");

                    for (result, expected) in job_results.result.into_iter().zip(expected_outputs.into_iter()) {
                        assert_eq!(result, expected);
                    }
                } else {
                    gadget_sdk::info!("No expected outputs specified, skipping signing verification");
                }
            },
        );
    };
}
