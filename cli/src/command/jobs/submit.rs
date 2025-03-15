use blueprint_core::info;
use color_eyre::Result;
use dialoguer::console::style;
use gadget_clients::tangle::client::OnlineClient;
use gadget_crypto::sp_core::SpSr25519;
use gadget_crypto::tangle_pair_signer::TanglePairSigner;
use gadget_keystore::backends::Backend;
use gadget_keystore::{Keystore, KeystoreConfig};
use tangle_subxt::subxt::tx::Signer;
use tangle_subxt::tangle_testnet_runtime::api;
use tangle_subxt::tangle_testnet_runtime::api::services::events::JobCalled;

use crate::command::jobs::check::check_job;
use crate::command::jobs::helpers::{load_job_args_from_file, prompt_for_job_params};
use crate::decode_bounded_string;

/// Submits a job to a service.
///
/// # Arguments
///
/// * `ws_rpc_url` - WebSocket RPC URL for the Tangle Network
/// * `service_id` - ID of the service to submit the job to
/// * `blueprint_id` - ID of the blueprint
/// * `keystore_uri` - URI for the keystore
/// * `job` - Job ID
/// * `params_file` - Optional path to a file containing job parameters
///
/// # Returns
///
/// Details of the job call.
///
/// # Errors
///
/// Returns an error if:
/// * Failed to connect to the Tangle Network
/// * Failed to sign or submit the transaction
/// * Transaction failed
/// * Blueprint not found
/// * Job ID not found in blueprint
/// * Parameter file not found or invalid
///
/// # Panics
///
/// Panics if:
/// * Failed to create keystore
/// * Failed to get keys from keystore
/// * Job was not called successfully
#[allow(clippy::too_many_lines)]
pub async fn submit_job(
    ws_rpc_url: String,
    service_id: Option<u64>,
    blueprint_id: u64,
    keystore_uri: String,
    // keystore_password: Option<String>, // TODO: Add keystore password support
    job: u8,
    params_file: Option<String>,
    watcher: bool,
) -> Result<JobCalled> {
    let client = OnlineClient::from_url(ws_rpc_url.clone()).await?;

    let config = KeystoreConfig::new().fs_root(keystore_uri.clone());
    let keystore = Keystore::new(config).expect("Failed to create keystore");
    let public = keystore.first_local::<SpSr25519>().unwrap();
    let pair = keystore.get_secret::<SpSr25519>(&public).unwrap();
    let signer = TanglePairSigner::new(pair.0);

    let service_id = service_id.unwrap();

    let blueprint_addr = tangle_subxt::tangle_testnet_runtime::api::storage()
        .services()
        .blueprints(blueprint_id);

    let blueprint_query = client
        .storage()
        .at_latest()
        .await?
        .fetch(&blueprint_addr)
        .await?
        .ok_or_else(|| color_eyre::eyre::eyre!("Blueprint not found"))?;

    let service_blueprint = blueprint_query.1;
    let blueprint_name = decode_bounded_string(&service_blueprint.metadata.name);
    println!(
        "{}",
        style(format!("Found blueprint: {}", blueprint_name)).green()
    );
    info!("Blueprint: {:?}", service_blueprint);

    // Get job arguments either from file or prompt
    let job_args = if let Some(file_path) = params_file {
        // Get job definition
        if job as usize >= service_blueprint.jobs.0.len() {
            return Err(color_eyre::eyre::eyre!(
                "Job ID {} not found in blueprint",
                job
            ));
        }
        let job_definition = &service_blueprint.jobs.0[job as usize];
        info!("Job definition: {:?}", job_definition);

        // Load arguments from file based on job definition
        load_job_args_from_file(&file_path, &job_definition.params.0)?
    } else {
        // Prompt user for arguments based on the job definition
        if job as usize >= service_blueprint.jobs.0.len() {
            return Err(color_eyre::eyre::eyre!(
                "Job ID {} not found in blueprint",
                job
            ));
        }

        let job_definition = &service_blueprint.jobs.0[job as usize];
        info!("Job definition: {:?}", job_definition);

        // Extract job name and description for better UX
        let job_name = decode_bounded_string(&job_definition.metadata.name);
        let job_description = job_definition
            .metadata
            .description
            .as_ref()
            .map(decode_bounded_string)
            .unwrap_or_default();

        println!(
            "{}",
            style(format!("Job: {} - {}", job_name, job_description))
                .cyan()
                .bold()
        );

        // Extract parameter types from the job definition
        let param_types = &job_definition.params.0;

        // Prompt for each parameter based on its type
        prompt_for_job_params(param_types)?
    };

    println!(
        "{}",
        style(format!(
            "Submitting job {} to service {} with {} parameters...",
            job,
            service_id,
            job_args.len()
        ))
        .cyan()
    );
    info!(
        "Submitting job {} to service {} with args: {:?}",
        job, service_id, job_args
    );
    let call = api::tx().services().call(service_id, job, job_args);
    let events = client
        .tx()
        .sign_and_submit_then_watch_default(&call, &signer)
        .await?
        .wait_for_finalized_success()
        .await?;

    let job_called_events = events.find::<JobCalled>().collect::<Vec<_>>();
    for job_called in job_called_events {
        let job_called = job_called?;
        if job_called.service_id == service_id
            && job_called.job == job
            && signer.account_id() == job_called.caller
        {
            println!(
                "{}",
                style(format!(
                    "Job {} successfully called on service {} with job ID: {} and call ID: {}",
                    job, service_id, job_called.job, job_called.call_id
                ))
                .green()
                .bold()
            );

            let result_types = service_blueprint.jobs.0[job as usize].result.0.clone();

            info!("Job {} successfully called on service {}", job, service_id);
            if watcher {
                check_job(ws_rpc_url, service_id, job_called.call_id, result_types, 0).await?;
            }
            return Ok(job_called);
        }
    }
    panic!("Job was not called");
}
