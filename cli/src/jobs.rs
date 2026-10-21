use crate::transaction_output::log_tx;
use alloy_primitives::Bytes;
use blueprint_client_tangle::TangleClient;
use cargo_tangle::command::jobs::check::wait_for_job_result;
use cargo_tangle::command::jobs::helpers::{
    JobSchema, list_jobs, load_job_call_details, load_job_schema, print_job_call_details,
    print_job_summaries,
};
use cargo_tangle::command::jobs::submit::submit_job as submit_job_call;
use cargo_tangle::command::tangle::TangleClientArgs;
use clap::Subcommand;
use color_eyre::eyre::{Context, Result, eyre};
use serde_json::json;
use std::{fs, path::PathBuf, time::Duration};

#[derive(Subcommand, Debug)]
pub(super) enum JobsCommands {
    /// List all jobs defined by a blueprint.
    ///
    /// Shows job indices, names, input schemas, and output types.
    List {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Blueprint ID to list jobs for.
        #[arg(long)]
        blueprint_id: u64,
        /// Output as JSON instead of formatted table.
        #[arg(long)]
        json: bool,
    },
    /// Show details for a submitted job call.
    ///
    /// Displays inputs, outputs, status, and result data.
    Show {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Blueprint ID the service belongs to.
        #[arg(long)]
        blueprint_id: u64,
        /// Service ID the job was submitted to.
        #[arg(long)]
        service_id: u64,
        /// Call ID returned when the job was submitted.
        #[arg(long)]
        call_id: u64,
        /// Output as JSON instead of formatted display.
        #[arg(long)]
        json: bool,
    },
    /// Submit a job to a running service.
    ///
    /// Invokes a job on the service operators. Inputs can be provided as:
    /// - Raw hex bytes (--payload-hex)
    /// - Binary file (--payload-file)
    /// - Structured JSON matching the job schema (--params-file)
    /// - Interactive prompts (--prompt)
    Submit {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Blueprint ID the service belongs to.
        #[arg(long)]
        blueprint_id: u64,
        /// Service ID to submit the job to.
        #[arg(long)]
        service_id: u64,
        /// Job index (0-based) as defined in the blueprint.
        #[arg(long)]
        job: u8,
        /// Job inputs as hex-encoded bytes (without 0x prefix).
        #[arg(long = "payload-hex", value_name = "HEX")]
        payload_hex: Option<String>,
        /// File containing raw job input bytes.
        #[arg(long = "payload-file", value_name = "FILE")]
        payload_file: Option<PathBuf>,
        /// JSON file with structured inputs matching the job schema.
        #[arg(
            long = "params-file",
            value_name = "FILE",
            conflicts_with_all = ["payload_hex", "payload_file"]
        )]
        params_file: Option<PathBuf>,
        /// Interactively prompt for each job input.
        #[arg(
            long,
            conflicts_with_all = ["payload_hex", "payload_file", "params_file"],
            action = clap::ArgAction::SetTrue
        )]
        prompt: bool,
        /// Wait for job result after submission.
        #[arg(long)]
        watch: bool,
        /// Timeout in seconds when watching for result.
        #[arg(long, default_value_t = 60)]
        timeout_secs: u64,
        /// Output transaction details as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Wait for a job result by call ID.
    ///
    /// Polls for job completion and displays the result when available.
    Watch {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Blueprint ID the service belongs to.
        #[arg(long)]
        blueprint_id: u64,
        /// Service ID the job was submitted to.
        #[arg(long)]
        service_id: u64,
        /// Call ID from the job submission.
        #[arg(long)]
        call_id: u64,
        /// Timeout in seconds before giving up.
        #[arg(long, default_value_t = 60)]
        timeout_secs: u64,
    },
}

pub(super) async fn execute(command: JobsCommands) -> Result<()> {
    match command {
        JobsCommands::List {
            network,
            blueprint_id,
            json,
        } => {
            let client = network.connect(blueprint_id, None).await?;
            let jobs = list_jobs(&client, blueprint_id).await?;
            print_job_summaries(&jobs, json);
        }
        JobsCommands::Show {
            network,
            blueprint_id,
            service_id,
            call_id,
            json,
        } => {
            let client = network.connect(blueprint_id, Some(service_id)).await?;
            let details = load_job_call_details(&client, blueprint_id, service_id, call_id).await?;
            print_job_call_details(&details, json);
        }
        JobsCommands::Submit {
            network,
            blueprint_id,
            service_id,
            job,
            payload_hex,
            payload_file,
            params_file,
            prompt,
            watch,
            timeout_secs,
            json,
        } => {
            let client = network.connect(blueprint_id, Some(service_id)).await?;
            let mut schema_cache: Option<JobSchema> = None;

            let payload = match (payload_hex, payload_file, params_file, prompt) {
                (Some(hex_value), None, None, false) => decode_payload_hex(&hex_value)?,
                (None, Some(path), None, false) => read_payload_file(&path)?,
                (None, None, Some(path), false) => {
                    ensure_schema_loaded(&mut schema_cache, &client, blueprint_id, job).await?;
                    schema_cache
                        .as_ref()
                        .expect("schema should be loaded")
                        .encode_params_from_file(&path)?
                }
                (None, None, None, true) => {
                    ensure_schema_loaded(&mut schema_cache, &client, blueprint_id, job).await?;
                    schema_cache
                        .as_ref()
                        .expect("schema should be loaded")
                        .prompt_for_params()?
                }
                (Some(_), Some(_), _, _) => {
                    return Err(eyre!(
                        "Specify only one of --payload-hex, --payload-file, --params-file, or --prompt"
                    ));
                }
                _ => {
                    return Err(eyre!(
                        "Provide job inputs via --payload-hex, --payload-file, --params-file, or --prompt"
                    ));
                }
            };

            let submission = submit_job_call(&client, service_id, job, payload.clone()).await?;
            log_tx("Job submission", &submission.tx, json);
            if json {
                println!(
                    "{}",
                    json!({
                        "event": "job_submitted",
                        "service_id": service_id,
                        "blueprint_id": blueprint_id,
                        "job": job,
                        "call_id": submission.call_id,
                        "tx_hash": format!("{:#x}", submission.tx.tx_hash),
                    })
                );
            } else {
                println!(
                    "Submitted job {job} to service {service_id}. Call ID: {} (tx: {:#x})",
                    submission.call_id, submission.tx.tx_hash
                );
            }
            if watch {
                let bytes = wait_for_job_result(
                    &client,
                    service_id,
                    submission.call_id,
                    Duration::from_secs(timeout_secs),
                )
                .await?;
                match ensure_schema_loaded(&mut schema_cache, &client, blueprint_id, job).await {
                    Ok(()) => {
                        let schema = schema_cache.as_ref().expect("schema present");
                        match schema.decode_and_format_results(&bytes) {
                            Ok(Some(lines)) => {
                                if json {
                                    println!(
                                        "{}",
                                        json!({
                                            "event": "job_result",
                                            "service_id": service_id,
                                            "call_id": submission.call_id,
                                            "decoded": lines,
                                            "length": bytes.len(),
                                        })
                                    );
                                } else {
                                    println!(
                                        "Job result ready ({} bytes). Decoded output:",
                                        bytes.len()
                                    );
                                    for line in lines {
                                        println!("  {line}");
                                    }
                                }
                            }
                            Ok(None) => print_raw_job_result(&bytes),
                            Err(err) => {
                                eprintln!("Failed to decode job result: {err}");
                                print_raw_job_result(&bytes);
                            }
                        }
                    }
                    Err(err) => {
                        eprintln!("Unable to load job schema: {err}");
                        print_raw_job_result(&bytes);
                    }
                }
            }
        }
        JobsCommands::Watch {
            network,
            blueprint_id,
            service_id,
            call_id,
            timeout_secs,
        } => {
            let client = network.connect(blueprint_id, Some(service_id)).await?;
            let bytes = wait_for_job_result(
                &client,
                service_id,
                call_id,
                Duration::from_secs(timeout_secs),
            )
            .await?;
            print_raw_job_result(&bytes);
        }
    }
    Ok(())
}

async fn ensure_schema_loaded(
    cache: &mut Option<JobSchema>,
    client: &TangleClient,
    blueprint_id: u64,
    job_index: u8,
) -> Result<()> {
    if cache.is_none() {
        let schema = load_job_schema(client, blueprint_id, job_index).await?;
        *cache = Some(schema);
    }
    Ok(())
}

fn decode_payload_hex(value: &str) -> Result<Bytes> {
    let trimmed = value.trim();
    let raw = trimmed.trim_start_matches("0x");
    let bytes = hex::decode(raw).context("invalid payload hex")?;
    Ok(Bytes::from(bytes))
}

fn read_payload_file(path: &PathBuf) -> Result<Bytes> {
    let data = fs::read(path)
        .with_context(|| format!("Failed to read payload file {}", path.display()))?;
    Ok(Bytes::from(data))
}

fn print_raw_job_result(bytes: &[u8]) {
    println!(
        "Job result ready ({} bytes): 0x{}",
        bytes.len(),
        hex::encode(bytes)
    );
}
