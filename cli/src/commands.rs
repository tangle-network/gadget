#![allow(clippy::too_many_arguments)]
use color_eyre::Result;
use gadget_chain_setup::tangle::InputValue;
use gadget_clients::tangle::client::{BlueprintId, OnlineClient};
use gadget_crypto::sp_core::SpSr25519;
use gadget_crypto::tangle_pair_signer::TanglePairSigner;
use gadget_keystore::{Keystore, KeystoreConfig};
use gadget_logging::info;
use gadget_blueprint_serde::{new_bounded_string, Field};
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::sp_arithmetic::per_things::Percent;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::{BoundedString, FieldType};
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::service::ServiceBlueprint;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::types::{
    Asset, AssetSecurityCommitment, AssetSecurityRequirement, MembershipModel,
};
use tangle_subxt::subxt::tx::{Signer, TxProgress};
use tangle_subxt::subxt::Config;
use tangle_subxt::subxt::blocks::ExtrinsicEvents;
use tangle_subxt::subxt::client::OnlineClientT;
use tangle_subxt::subxt::utils::AccountId32;
use tangle_subxt::tangle_testnet_runtime::api;
use tangle_subxt::tangle_testnet_runtime::api::assets::events::created::AssetId;
use tangle_subxt::tangle_testnet_runtime::api::services::events::JobCalled;
use gadget_keystore::backends::Backend;
use gadget_utils_tangle::TxProgressExt;
use serde_json;

pub async fn list_requests(ws_rpc_url: String) -> Result<()> {
    let client = OnlineClient::from_url(ws_rpc_url.clone()).await?;

    let service_requests_addr = tangle_subxt::tangle_testnet_runtime::api::storage()
        .services()
        .service_requests_iter();

    let mut storage_query = client
        .storage()
        .at_latest()
        .await?
        .iter(service_requests_addr)
        .await?;

    let mut requests = Vec::new();

    gadget_logging::info!("Fetching service requests...");

    while let Some(result) = storage_query.next().await {
        let result = result?;
        let request = result.value;
        let id = u64::from_le_bytes(result.key_bytes[32..].try_into().unwrap());
        requests.push((id, request));
    }

    if requests.is_empty() {
        gadget_logging::info!("No service requests found");
        return Ok(());
    }

    println!("\nService Requests");
    println!("=============================================");

    for (request_id, request) in requests {
        println!("Request ID: {}", request_id);
        println!("Blueprint: {}", request.blueprint);
        println!("Owner: {}", request.owner);
        println!("Permitted Callers: {:?}", request.permitted_callers);
        println!("Security Requirements: {:?}", request.security_requirements);
        println!("Membership Model: {:?}", request.membership_model);
        println!("Request Arguments: {:?}", request.args);
        println!("TTL: {:?}", request.ttl);
        println!("=============================================");
    }

    Ok(())
}

pub async fn accept_request(
    ws_rpc_url: String,
    _min_exposure_percent: u8,
    _max_exposure_percent: u8,
    restaking_percent: u8,
    keystore_uri: String,
    // keystore_password: Option<String>, // TODO: Add keystore password support
    request_id: u64,
) -> Result<()> {
    let client = OnlineClient::from_url(ws_rpc_url.clone()).await?;

    let config = KeystoreConfig::new().fs_root(keystore_uri.clone());
    let keystore = Keystore::new(config).expect("Failed to create keystore");
    let public = keystore.first_local::<SpSr25519>().unwrap();
    let pair = keystore.get_secret::<SpSr25519>(&public).unwrap();
    let signer = TanglePairSigner::new(pair.0);

    let native_security_commitments =
        vec![get_security_commitment(Asset::Custom(0), restaking_percent)];

    let call = tangle_subxt::tangle_testnet_runtime::api::tx()
        .services()
        .approve(request_id, native_security_commitments);
    info!("Submitting Service Approval for request ID: {}", request_id);
    let res = client
        .tx()
        .sign_and_submit_then_watch_default(&call, &signer)
        .await?;
    wait_for_in_block_success(res).await;
    info!("Service Approval submitted successfully");
    Ok(())
}

pub async fn reject_request(
    ws_rpc_url: String,
    keystore_uri: String,
    // keystore_password: Option<String>, // TODO: Add keystore password support
    request_id: u64,
) -> Result<()> {
    let client = OnlineClient::from_url(ws_rpc_url.clone()).await?;

    let config = KeystoreConfig::new().fs_root(keystore_uri.clone());
    let keystore = Keystore::new(config).expect("Failed to create keystore");
    let public = keystore.first_local::<SpSr25519>().unwrap();
    let pair = keystore.get_secret::<SpSr25519>(&public).unwrap();
    let signer = TanglePairSigner::new(pair.0);

    let call = tangle_subxt::tangle_testnet_runtime::api::tx()
        .services()
        .reject(request_id);
    let res = client
        .tx()
        .sign_and_submit_then_watch_default(&call, &signer)
        .await?;
    wait_for_in_block_success(res).await;
    Ok(())
}

pub async fn request_service(
    ws_rpc_url: String,
    blueprint_id: u64,
    min_exposure_percent: u8,
    max_exposure_percent: u8,
    target_operators: Vec<AccountId32>,
    value: u128,
    keystore_uri: String,
    // keystore_password: Option<String>, // TODO: Add keystore password support
) -> Result<()> {
    let client = OnlineClient::from_url(ws_rpc_url.clone()).await?;

    let config = KeystoreConfig::new().fs_root(keystore_uri.clone());
    let keystore = Keystore::new(config).expect("Failed to create keystore");
    let public = keystore.first_local::<SpSr25519>().unwrap();
    let pair = keystore.get_secret::<SpSr25519>(&public).unwrap();
    let signer = TanglePairSigner::new(pair.0);

    let min_operators = target_operators.len() as u32;
    let security_requirements = vec![AssetSecurityRequirement {
        asset: Asset::Custom(0),
        min_exposure_percent: Percent(min_exposure_percent),
        max_exposure_percent: Percent(max_exposure_percent),
    }];
    let call = tangle_subxt::tangle_testnet_runtime::api::tx()
        .services()
        .request(
            None,
            blueprint_id as BlueprintId,
            Vec::new(),
            target_operators,
            Default::default(),
            security_requirements,
            1000,
            Asset::Custom(0),
            value,
            MembershipModel::Fixed { min_operators },
        );
    info!("Submitting Service Request...");
    let res = client
        .tx()
        .sign_and_submit_then_watch_default(&call, &signer)
        .await?;
    wait_for_in_block_success(res).await;
    info!("Service Request submitted successfully");
    Ok(())
}

pub async fn submit_job(
    ws_rpc_url: String,
    service_id: Option<u64>,
    blueprint_id: u64,
    keystore_uri: String,
    // keystore_password: Option<String>, // TODO: Add keystore password support
    job: u8,
    params_file: Option<String>,
) -> Result<()> {
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
    info!("Blueprint: {:?}", service_blueprint);

    // Get job arguments either from file or prompt
    let job_args = if let Some(file_path) = params_file {
        // Load arguments from file
        load_job_args_from_file(&file_path)?
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
            .map(|desc| decode_bounded_string(desc))
            .unwrap_or_default();

        println!("Job: {} - {}", job_name, job_description);

        // Extract parameter types from the job definition
        let param_types = &job_definition.params.0;

        // Prompt for each parameter based on its type
        prompt_for_job_params(param_types)?
    };

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
            info!("Job {} successfully called on service {}", job, service_id);
            return Ok(());
        }
    }
    panic!("Job was not called");
}

/// Helper function to decode a BoundedString to a regular String
fn decode_bounded_string(bounded_string: &BoundedString) -> String {
    String::from_utf8_lossy(&bounded_string.0 .0).to_string()
}

/// Load job arguments from a JSON file
fn load_job_args_from_file(file_path: &str) -> Result<Vec<InputValue>> {
    use std::fs;
    use std::path::Path;

    let path = Path::new(file_path);
    if !path.exists() {
        return Err(color_eyre::eyre::eyre!(
            "Parameters file not found: {}",
            file_path
        ));
    }

    let content = fs::read_to_string(path)?;
    let args: Vec<String> = serde_json::from_str(&content)?;

    // Parse each argument string into an InputValue
    let mut input_values = Vec::new();
    for arg in args {
        let input_value = input_value_parser(&arg)
            .map_err(|e| color_eyre::eyre::eyre!("Failed to parse argument from file: {}", e))?;
        input_values.push(input_value);
    }

    Ok(input_values)
}

/// Prompt the user for job parameters based on the parameter types
fn prompt_for_job_params(param_types: &Vec<FieldType>) -> Result<Vec<InputValue>> {
    use dialoguer::{Input, Select};

    let mut args = Vec::new();

    for (i, param_type) in param_types.iter().enumerate() {
        println!("Parameter {}: {:?}", i + 1, param_type);

        match param_type {
            FieldType::Uint8 => {
                let value: u8 = Input::new()
                    .with_prompt(format!("Enter u8 value for parameter {}", i + 1))
                    .interact()?;
                args.push(InputValue::Uint8(value));
            }
            FieldType::Uint16 => {
                let value: u16 = Input::new()
                    .with_prompt(format!("Enter u16 value for parameter {}", i + 1))
                    .interact()?;
                args.push(InputValue::Uint16(value));
            }
            FieldType::Uint32 => {
                let value: u32 = Input::new()
                    .with_prompt(format!("Enter u32 value for parameter {}", i + 1))
                    .interact()?;
                args.push(InputValue::Uint32(value));
            }
            FieldType::Uint64 => {
                let value: u64 = Input::new()
                    .with_prompt(format!("Enter u64 value for parameter {}", i + 1))
                    .interact()?;
                args.push(InputValue::Uint64(value));
            }
            FieldType::Int8 => {
                let value: i8 = Input::new()
                    .with_prompt(format!("Enter i8 value for parameter {}", i + 1))
                    .interact()?;
                args.push(InputValue::Int8(value));
            }
            FieldType::Int16 => {
                let value: i16 = Input::new()
                    .with_prompt(format!("Enter i16 value for parameter {}", i + 1))
                    .interact()?;
                args.push(InputValue::Int16(value));
            }
            FieldType::Int32 => {
                let value: i32 = Input::new()
                    .with_prompt(format!("Enter i32 value for parameter {}", i + 1))
                    .interact()?;
                args.push(InputValue::Int32(value));
            }
            FieldType::Int64 => {
                let value: i64 = Input::new()
                    .with_prompt(format!("Enter i64 value for parameter {}", i + 1))
                    .interact()?;
                args.push(InputValue::Int64(value));
            }
            FieldType::Bool => {
                let value: bool = Input::new()
                    .with_prompt(format!(
                        "Enter boolean value (true/false) for parameter {}",
                        i + 1
                    ))
                    .interact()?;
                args.push(InputValue::Bool(value));
            }
            FieldType::String => {
                let value: String = Input::new()
                    .with_prompt(format!("Enter string value for parameter {}", i + 1))
                    .interact()?;
                args.push(InputValue::String(new_bounded_string(value)));
            }
            _ => {
                return Err(color_eyre::eyre::eyre!(
                    "Unsupported parameter type: {:?}",
                    param_type
                ));
            }
        }
    }

    Ok(args)
}

/// Prompt the user for job arguments based on the blueprint
fn prompt_for_job_args(job_id: u8, blueprint: &ServiceBlueprint) -> Result<Vec<InputValue>> {
    use dialoguer::{Input, Select};

    println!("Enter arguments for job {}:", job_id);

    // In a real implementation, we would extract job parameter information from the blueprint
    // For now, we'll just prompt for the number of arguments and their types

    let num_args: usize = Input::new()
        .with_prompt("Number of arguments")
        .default(0)
        .interact()?;

    let mut args = Vec::new();

    for i in 0..num_args {
        // Prompt for argument type
        let type_options = &[
            "Uint8", "Uint16", "Uint32", "Uint64", "Uint128", "Int8", "Int16", "Int32", "Int64",
            "Int128", "Bool", "Bytes", "String",
        ];

        let type_idx = Select::new()
            .with_prompt(format!("Select type for argument {}", i + 1))
            .items(type_options)
            .default(3) // Default to Uint64
            .interact()?;

        // Prompt for value based on selected type
        match type_options[type_idx] {
            "Uint8" => {
                let value: u8 = Input::new().with_prompt("Enter value").interact()?;
                args.push(InputValue::Uint8(value));
            }
            "Uint16" => {
                let value: u16 = Input::new().with_prompt("Enter value").interact()?;
                args.push(InputValue::Uint16(value));
            }
            "Uint32" => {
                let value: u32 = Input::new().with_prompt("Enter value").interact()?;
                args.push(InputValue::Uint32(value));
            }
            "Uint64" => {
                let value: u64 = Input::new().with_prompt("Enter value").interact()?;
                args.push(InputValue::Uint64(value));
            }
            // "Uint128" => {
            //     let value: String = Input::new()
            //         .with_prompt("Enter value")
            //         .interact()?;
            //     let value = value.parse::<u128>()
            //         .map_err(|e| color_eyre::eyre::eyre!("Failed to parse u128: {}", e))?;
            //     args.push(InputValue::(value));
            // },
            "Int8" => {
                let value: i8 = Input::new().with_prompt("Enter value").interact()?;
                args.push(InputValue::Int8(value));
            }
            "Int16" => {
                let value: i16 = Input::new().with_prompt("Enter value").interact()?;
                args.push(InputValue::Int16(value));
            }
            "Int32" => {
                let value: i32 = Input::new().with_prompt("Enter value").interact()?;
                args.push(InputValue::Int32(value));
            }
            "Int64" => {
                let value: i64 = Input::new().with_prompt("Enter value").interact()?;
                args.push(InputValue::Int64(value));
            }
            // "Int128" => {
            //     let value: String = Input::new()
            //         .with_prompt("Enter value")
            //         .interact()?;
            //     let value = value.parse::<i128>()
            //         .map_err(|e| color_eyre::eyre::eyre!("Failed to parse i128: {}", e))?;
            //     args.push(InputValue::Int128(value));
            // },
            "Bool" => {
                let value: bool = Input::new()
                    .with_prompt("Enter value (true/false)")
                    .interact()?;
                args.push(InputValue::Bool(value));
            }
            // "Bytes" => {
            //     let value: String = Input::new()
            //         .with_prompt("Enter hex bytes (without 0x prefix)")
            //         .interact()?;
            //     let bytes = hex::decode(&value)
            //         .map_err(|e| color_eyre::eyre::eyre!("Failed to parse hex bytes: {}", e))?;
            //     args.push(InputValue::Bytes(bytes.into()));
            // },
            "String" => {
                let value: String = Input::new().with_prompt("Enter string value").interact()?;
                args.push(InputValue::String(new_bounded_string(value)));
            }
            _ => unreachable!(),
        }
    }

    Ok(args)
}

/// Helper function to parse InputValue from string (for use in main.rs)
pub fn input_value_parser(s: &str) -> Result<InputValue, String> {
    // Parse the input string based on its format
    if s.starts_with("u8:") {
        // Parse as Uint8
        let value = s
            .trim_start_matches("u8:")
            .parse::<u8>()
            .map_err(|e| format!("Failed to parse u8 value: {}", e))?;
        Ok(InputValue::Uint8(value))
    } else if s.starts_with("u16:") {
        // Parse as Uint16
        let value = s
            .trim_start_matches("u16:")
            .parse::<u16>()
            .map_err(|e| format!("Failed to parse u16 value: {}", e))?;
        Ok(InputValue::Uint16(value))
    } else if s.starts_with("u32:") {
        // Parse as Uint32
        let value = s
            .trim_start_matches("u32:")
            .parse::<u32>()
            .map_err(|e| format!("Failed to parse u32 value: {}", e))?;
        Ok(InputValue::Uint32(value))
    } else if s.starts_with("u64:") {
        // Parse as Uint64
        let value = s
            .trim_start_matches("u64:")
            .parse::<u64>()
            .map_err(|e| format!("Failed to parse u64 value: {}", e))?;
        Ok(InputValue::Uint64(value))
    } else if s.starts_with("i8:") {
        // Parse as Int8
        let value = s
            .trim_start_matches("i8:")
            .parse::<i8>()
            .map_err(|e| format!("Failed to parse i8 value: {}", e))?;
        Ok(InputValue::Int8(value))
    } else if s.starts_with("i16:") {
        // Parse as Int16
        let value = s
            .trim_start_matches("i16:")
            .parse::<i16>()
            .map_err(|e| format!("Failed to parse i16 value: {}", e))?;
        Ok(InputValue::Int16(value))
    } else if s.starts_with("i32:") {
        // Parse as Int32
        let value = s
            .trim_start_matches("i32:")
            .parse::<i32>()
            .map_err(|e| format!("Failed to parse i32 value: {}", e))?;
        Ok(InputValue::Int32(value))
    } else if s.starts_with("i64:") {
        // Parse as Int64
        let value = s
            .trim_start_matches("i64:")
            .parse::<i64>()
            .map_err(|e| format!("Failed to parse i64 value: {}", e))?;
        Ok(InputValue::Int64(value))
    } else if s.starts_with("bool:") {
        // Parse as Bool
        let value = s
            .trim_start_matches("bool:")
            .parse::<bool>()
            .map_err(|e| format!("Failed to parse bool value: {}", e))?;
        Ok(InputValue::Bool(value))
    } else if s.starts_with("string:") {
        // Parse as String
        let value = s.trim_start_matches("string:").to_string();
        Ok(InputValue::String(new_bounded_string(value)))
    } else {
        // Default to u64 if no prefix is provided
        match s.parse::<u64>() {
            Ok(value) => Ok(InputValue::Uint64(value)),
            Err(_) => Err(format!(
                "Failed to parse input value: {}. Use prefix like 'u64:', 'string:', etc.",
                s
            )),
        }
    }
}

pub async fn register(
    ws_rpc_url: String,
    blueprint_id: u64,
    keystore_uri: String,
    // keystore_password: Option<String>, // TODO: Add keystore password support
) -> Result<()> {
    let client = OnlineClient::from_url(ws_rpc_url.clone()).await?;

    let config = KeystoreConfig::new().fs_root(keystore_uri.clone());
    let keystore = Keystore::new(config).expect("Failed to create keystore");
    let public = keystore.first_local::<SpSr25519>().unwrap();
    let pair = keystore.get_secret::<SpSr25519>(&public).unwrap();
    let signer = TanglePairSigner::new(pair.0);

    let ecdsa_public = keystore
        .first_local::<gadget_crypto::sp_core::SpEcdsa>()
        .map_err(|e| color_eyre::eyre::eyre!("Missing ECDSA key: {}", e))?;

    let preferences =
        tangle_subxt::tangle_testnet_runtime::api::services::calls::types::register::Preferences {
            key: gadget_runners::tangle::tangle::decompress_pubkey(&ecdsa_public.0 .0).unwrap(),
            price_targets: gadget_runners::tangle::tangle::PriceTargets::default().0,
        };

    info!("Joining operators...");
    let join_call = api::tx()
        .multi_asset_delegation()
        .join_operators(1_000_000_000_000_000);
    let join_res = client
        .tx()
        .sign_and_submit_then_watch_default(&join_call, &signer)
        .await?;

    // Wait for finalization instead of just in-block
    let events = join_res.wait_for_finalized_success().await?;
    info!("Successfully joined operators with events: {:?}", events);

    info!("Registering for blueprint {}...", blueprint_id);
    let registration_args = tangle_subxt::tangle_testnet_runtime::api::services::calls::types::register::RegistrationArgs::new();
    let register_call =
        api::tx()
            .services()
            .register(blueprint_id, preferences, registration_args, 0);
    let register_res = client
        .tx()
        .sign_and_submit_then_watch_default(&register_call, &signer)
        .await?;

    // Wait for finalization instead of just in-block
    let events = register_res.wait_for_finalized_success().await?;
    info!(
        "Successfully registered for blueprint with ID: {} with events: {:?}",
        blueprint_id, events
    );

    // Verify registration by querying the latest block
    info!("Verifying registration...");
    let latest_block = client.blocks().at_latest().await?;
    let latest_block_hash = latest_block.hash();
    info!("Latest block: {:?}", latest_block.number());

    // Create a TangleServicesClient to query operator blueprints
    let services_client =
        gadget_clients::tangle::services::TangleServicesClient::new(client.clone());

    // Get the account ID from the signer
    let account_id = signer.account_id();
    info!("Querying blueprints for account: {:?}", account_id);

    // Query operator blueprints at the latest block
    let block_hash = latest_block_hash.0;
    let blueprints = services_client
        .query_operator_blueprints(block_hash, account_id.clone())
        .await?;

    info!("Found {} blueprints for operator", blueprints.len());
    for (i, blueprint) in blueprints.iter().enumerate() {
        info!("Blueprint {}: {:?}", i, blueprint);
    }

    Ok(())
}

async fn wait_for_in_block_success<T: Config>(
    res: TxProgress<T, impl OnlineClientT<T>>,
) -> ExtrinsicEvents<T> {
    res.wait_for_in_block()
        .await
        .unwrap()
        .fetch_events()
        .await
        .unwrap()
}

fn get_security_commitment(a: Asset<AssetId>, p: u8) -> AssetSecurityCommitment<AssetId> {
    AssetSecurityCommitment {
        asset: a,
        exposure_percent: Percent(p),
    }
}
