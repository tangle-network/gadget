use crate::runtime_options::resolve_runtime_options;
use crate::staking_inputs::{
    MAX_BPS, SecurityCommitmentArg, SecurityRequirementArg, normalize_operator_exposures,
    parse_address_list, parse_commitments, parse_security_commitment, parse_security_requirement,
    requirement_to_abi,
};
use crate::transaction_output::log_tx;
use crate::upgrade_context::{upgrade_tx_ctx, upgrade_view_ctx};
use alloy_primitives::{Bytes, U256};
use blueprint_client_tangle::contracts::ITangleTypes;
use cargo_tangle::command::run::tangle::{RunOpts, run_blueprint};
use cargo_tangle::command::service::{
    approve_service, approve_service_with_commitments, build_request_params, join_service,
    leave_service, reject_service, request_service, with_security_requirements,
};
use cargo_tangle::command::tangle::{
    PreferredSourceArg, SpawnMethod, TangleClientArgs, parse_address,
};
use cargo_tangle::command::{list, operator};
use cargo_tangle::settings::RuntimePreferences;
use clap::Subcommand;
use color_eyre::eyre::{Context, Result, ensure, eyre};
use serde_json::json;
use std::{fs, path::PathBuf};
use url::Url;

#[derive(Subcommand, Debug)]
pub(super) enum ServiceCommands {
    /// Request a new service instance from operators.
    ///
    /// Creates a service request that operators can approve or reject.
    /// Payment and security requirements can be specified to filter operators.
    Request {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Blueprint ID to instantiate.
        #[arg(long)]
        blueprint_id: u64,
        /// Operator addresses to include (can specify multiple times).
        #[arg(long = "operator", required = true)]
        operators: Vec<String>,
        /// Exposure per operator in basis points (10000 = 100%). Matches --operator order.
        #[arg(long = "operator-exposure-bps")]
        operator_exposures: Vec<u16>,
        /// Addresses allowed to submit jobs (in addition to requester).
        #[arg(long = "permitted-caller")]
        permitted_callers: Vec<String>,
        /// File containing service configuration (raw bytes).
        #[arg(long = "config-file", value_name = "PATH")]
        config_file: Option<PathBuf>,
        /// Hex-encoded service configuration.
        #[arg(long = "config-hex", value_name = "HEX")]
        config_hex: Option<String>,
        /// Time-to-live in seconds (0 = no expiration).
        #[arg(long, default_value_t = 600)]
        ttl: u64,
        /// ERC20 token for payment (0x0 = native token).
        #[arg(long, default_value = "0x0000000000000000000000000000000000000000")]
        payment_token: String,
        /// Payment amount in wei.
        #[arg(long, default_value_t = 0)]
        payment_amount: u128,
        /// Security requirement (format: KIND:TOKEN:MIN:MAX, can repeat).
        ///
        /// KIND: 0=native, 1=erc20. MIN/MAX are stake bounds in wei.
        #[arg(
            long = "security-requirement",
            value_name = "KIND:TOKEN:MIN:MAX",
            value_parser = parse_security_requirement
        )]
        security_requirements: Vec<SecurityRequirementArg>,
        /// Output transaction details as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Approve a pending service request as an operator.
    ///
    /// Commits your stake to the service and enables job execution.
    Approve {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Request ID to approve.
        #[arg(long)]
        request_id: u64,
        /// Percentage of your stake to commit to this service (0-100).
        #[arg(long = "staking-percent", default_value_t = 50)]
        staking_percent: u8,
        /// Explicit security commitment (format: KIND:TOKEN:EXPOSURE, can repeat).
        ///
        /// Overrides automatic allocation. EXPOSURE is in wei.
        #[arg(
            long = "security-commitment",
            value_name = "KIND:TOKEN:EXPOSURE",
            value_parser = parse_security_commitment
        )]
        security_commitments: Vec<SecurityCommitmentArg>,
        /// Output transaction details as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Reject a pending service request as an operator.
    ///
    /// Declines participation in the service. If all operators reject,
    /// the request fails and payment is refunded to the requester.
    Reject {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Request ID to reject.
        #[arg(long)]
        request_id: u64,
        /// Output transaction details as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Join a running dynamic service as an operator.
    ///
    /// For services with open membership, allows operators to join after creation.
    Join {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Service ID to join.
        #[arg(long)]
        service_id: u64,
        /// Stake exposure in basis points (10000 = 100% of your delegated stake).
        #[arg(long, default_value_t = MAX_BPS)]
        exposure_bps: u16,
        /// Asset security commitment in format KIND:TOKEN:EXPOSURE_BPS.
        /// KIND: native/eth or erc20.
        /// TOKEN: Token/vault address (use _ or 0 for native).
        /// EXPOSURE_BPS: Exposure in basis points (e.g., 5000 = 50%).
        /// Can be specified multiple times for multiple commitments.
        /// Example: --commitment erc20:0x1234...abcd:5000
        #[arg(long, value_name = "KIND:TOKEN:EXPOSURE_BPS")]
        commitment: Vec<String>,
        /// Output transaction details as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Leave a dynamic service as an operator.
    ///
    /// Exits the service and recovers your committed stake after the unbonding period.
    Leave {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Service ID to leave.
        #[arg(long)]
        service_id: u64,
        /// Output transaction details as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Spawn a local service runtime for testing.
    ///
    /// Starts the blueprint runtime locally without full network participation.
    /// Useful for development and debugging.
    Spawn {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Blueprint ID defining the service logic.
        #[arg(long)]
        blueprint_id: u64,
        /// Service ID to spawn runtime for.
        #[arg(long)]
        service_id: u64,
        /// Runtime execution mode: vm (sandboxed) or native.
        #[arg(long, value_enum, default_value_t = SpawnMethod::Vm)]
        spawn_method: SpawnMethod,
        /// Directory for blueprint data and state.
        #[arg(long, value_name = "PATH")]
        data_dir: Option<PathBuf>,
        /// Allow unchecked attestations (testing only, insecure).
        #[arg(long)]
        allow_unchecked_attestations: bool,
        /// Simulate execution without on-chain transactions.
        #[arg(long)]
        dry_run: bool,
        /// Override blueprint source: wasm, binary, or container.
        #[arg(long, value_enum)]
        preferred_source: Option<PreferredSourceArg>,
        /// Force VM sandbox execution.
        #[arg(long)]
        vm: bool,
        /// Disable VM sandbox (use native execution).
        #[arg(long)]
        no_vm: bool,
    },
    /// List all active services.
    List {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Output as JSON instead of formatted table.
        #[arg(long)]
        json: bool,
    },
    /// List all pending service requests.
    Requests {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Output as JSON instead of formatted table.
        #[arg(long)]
        json: bool,
    },
    /// Show details for a specific service request.
    Show {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Request ID to display.
        #[arg(long)]
        request_id: u64,
    },

    /// Set this service's upgrade policy: AUTO follows the blueprint owner's
    /// active version, APPROVE requires opt-in via `ack-version`, MANUAL pins
    /// to genesis.
    SetPolicy {
        #[command(flatten)]
        network: TangleClientArgs,
        #[arg(long)]
        service_id: u64,
        /// Policy to apply.
        #[arg(long, value_enum)]
        policy: cargo_tangle::command::upgrade::UpgradePolicyArg,
        #[arg(long)]
        json: bool,
    },

    /// Acknowledge a binary version for this service (under APPROVE policy).
    AckVersion {
        #[command(flatten)]
        network: TangleClientArgs,
        #[arg(long)]
        service_id: u64,
        #[arg(long)]
        version_id: u64,
        #[arg(long)]
        json: bool,
    },

    /// Show the binary version a service is currently effectively running.
    EffectiveVersion {
        #[command(flatten)]
        network: TangleClientArgs,
        #[arg(long)]
        service_id: u64,
        /// Blueprint ID hosting this service. Required to resolve the
        /// effective version's blueprint context.
        #[arg(long)]
        blueprint_id: u64,
        #[arg(long)]
        json: bool,
    },

    /// Show the service's upgrade policy + acked vs. available versions.
    UpgradeStatus {
        #[command(flatten)]
        network: TangleClientArgs,
        #[arg(long)]
        service_id: u64,
        /// Blueprint ID hosting this service.
        #[arg(long)]
        blueprint_id: u64,
        #[arg(long)]
        json: bool,
    },

    /// List the binary versions available for this service.
    ///
    /// Talks to the local blueprint-manager (not the chain) and asks it to
    /// enumerate the published versions for the underlying blueprint, with
    /// the currently-running one flagged. Useful as the entrypoint for
    /// MANUAL-with-assist workflows (`upgrade-local`, `upgrade-whitelist`).
    Upgrades {
        #[arg(long)]
        service_id: u64,
        /// blueprint-manager local RPC base URL. Falls back to
        /// `BLUEPRINT_MANAGER_URL` env, then `http://127.0.0.1:9000`.
        #[arg(long, value_name = "URL")]
        manager_url: Option<Url>,
        #[arg(long)]
        json: bool,
    },

    /// Pre-authorize the manager to swap this service to `--version-id`.
    ///
    /// MANUAL-with-assist: stores a one-shot pin in the manager's local
    /// authz store. The manager will run its full verify-then-swap pipeline
    /// the next time it reconciles this service, then clear the pin. No
    /// on-chain ack tx is written; the service's upgrade policy must be
    /// MANUAL for this to take effect.
    UpgradeLocal {
        #[arg(long)]
        service_id: u64,
        #[arg(long)]
        version_id: u64,
        /// Don't actually pin — just report what the manager would do.
        #[arg(long)]
        dry_run: bool,
        #[arg(long, value_name = "URL")]
        manager_url: Option<Url>,
        #[arg(long)]
        json: bool,
    },

    /// Replace this service's MANUAL-with-assist whitelist of acceptable
    /// versions. Persisted across manager restarts. Pass `--versions ""` to
    /// clear.
    UpgradeWhitelist {
        #[arg(long)]
        service_id: u64,
        /// Comma-separated version IDs (e.g. `2,4,5`). Empty = clear.
        #[arg(long, value_name = "LIST")]
        versions: String,
        #[arg(long, value_name = "URL")]
        manager_url: Option<Url>,
        #[arg(long)]
        json: bool,
    },

    /// Record an explicit skip on a published version for this service.
    /// The manager will suppress alerts about it and never auto-swap into
    /// it even if it lands in `effectiveBinaryVersion`. Persists across
    /// manager restarts.
    UpgradeSkip {
        #[arg(long)]
        service_id: u64,
        #[arg(long)]
        version_id: u64,
        /// Required free-form reason. Lands in the manager's audit log.
        #[arg(long, value_name = "TEXT")]
        reason: String,
        #[arg(long, value_name = "URL")]
        manager_url: Option<Url>,
        #[arg(long)]
        json: bool,
    },

    /// Show the local-authz state (pinned, whitelisted, skipped) for this
    /// service along with the on-chain policy and currently-running binary.
    UpgradeAuthz {
        #[arg(long)]
        service_id: u64,
        #[arg(long, value_name = "URL")]
        manager_url: Option<Url>,
        #[arg(long)]
        json: bool,
    },
}

pub(super) async fn execute(command: ServiceCommands) -> Result<()> {
    match command {
        ServiceCommands::Request {
            network,
            blueprint_id,
            operators,
            operator_exposures,
            permitted_callers,
            config_file,
            config_hex,
            ttl,
            payment_token,
            payment_amount,
            security_requirements,
            json,
        } => {
            let operators = parse_address_list(&operators, "operator")?;
            let operator_exposures =
                normalize_operator_exposures(&operator_exposures, operators.len())?;
            let permitted_callers = parse_address_list(&permitted_callers, "permitted caller")?;
            let config = load_config_payload(config_file, config_hex)?;
            let payment_token = parse_address(&payment_token, "PAYMENT_TOKEN")?;
            let security_requirements: Vec<ITangleTypes::AssetSecurityRequirement> =
                security_requirements
                    .into_iter()
                    .map(requirement_to_abi)
                    .collect();
            let client = network.connect(blueprint_id, None).await?;
            let params = with_security_requirements(
                build_request_params(
                    blueprint_id,
                    operators,
                    operator_exposures,
                    permitted_callers,
                    ttl,
                    payment_token,
                    U256::from(payment_amount),
                    config,
                ),
                security_requirements,
            );
            let (tx, request_id) = request_service(&client, params).await?;

            log_tx("Service request", &tx, json);
            if json {
                println!(
                    "{}",
                    json!({
                        "event": "service_request_id",
                        "request_id": request_id,
                        "tx_hash": format!("{:#x}", tx.tx_hash),
                    })
                );
            } else {
                println!("Request ID: {request_id}");
            }
        }
        ServiceCommands::Approve {
            network,
            request_id,
            staking_percent,
            json,
            security_commitments,
        } => {
            let client = network.connect(0, None).await?;
            let tx = if security_commitments.is_empty() {
                approve_service(&client, request_id, staking_percent).await?
            } else {
                let commitments: Vec<ITangleTypes::AssetSecurityCommitment> = security_commitments
                    .into_iter()
                    .map(commitment_to_abi)
                    .collect();
                approve_service_with_commitments(&client, request_id, commitments).await?
            };
            log_tx("Service approval", &tx, json);
        }
        ServiceCommands::Reject {
            network,
            request_id,
            json,
        } => {
            let client = network.connect(0, None).await?;
            let tx = reject_service(&client, request_id).await?;
            log_tx("Service rejection", &tx, json);
        }
        ServiceCommands::Join {
            network,
            service_id,
            exposure_bps,
            commitment,
            json,
        } => {
            ensure!(exposure_bps > 0, "Exposure must be greater than 0 bps");
            ensure!(
                exposure_bps <= MAX_BPS,
                "Exposure cannot exceed {MAX_BPS} bps"
            );
            let client = network.connect(0, Some(service_id)).await?;
            let tx = if commitment.is_empty() {
                join_service(&client, service_id, exposure_bps).await?
            } else {
                let commitments = parse_commitments(&commitment)?;
                client
                    .join_service_with_commitments(service_id, exposure_bps, commitments)
                    .await
                    .map_err(|e| eyre!(e.to_string()))?
            };
            log_tx("Service join", &tx, json);
            if json {
                println!(
                    "{}",
                    json!({
                        "event": "service_joined",
                        "service_id": service_id,
                        "exposure_bps": exposure_bps,
                        "tx_hash": format!("{:#x}", tx.tx_hash),
                    })
                );
            } else {
                println!("Joined service {service_id} with exposure {exposure_bps} bps");
            }
        }
        ServiceCommands::Leave {
            network,
            service_id,
            json,
        } => {
            let client = network.connect(0, Some(service_id)).await?;
            let operator = client.account();
            let operator_info = client
                .get_service_operator(service_id, operator)
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            ensure!(
                operator_info.active,
                "Operator is not active in service {service_id}"
            );

            let tx = leave_service(&client, service_id).await?;
            log_tx("Service leave", &tx, json);
            if json {
                println!(
                    "{}",
                    json!({
                        "event": "service_left",
                        "service_id": service_id,
                        "tx_hash": format!("{:#x}", tx.tx_hash),
                    })
                );
            } else {
                println!("Left service {service_id}");
            }
        }
        ServiceCommands::Spawn {
            network,
            blueprint_id,
            service_id,
            spawn_method,
            data_dir,
            allow_unchecked_attestations,
            dry_run,
            preferred_source,
            vm,
            no_vm,
        } => {
            let client_config = network.client_config(blueprint_id, Some(service_id))?;
            let settings = client_config.settings.clone();
            let (preferred_source_value, use_vm_value) = resolve_runtime_options(
                spawn_method,
                RuntimePreferences::default(),
                preferred_source,
                vm,
                no_vm,
            )?;
            let run_opts = RunOpts {
                http_rpc_url: client_config.http_rpc_endpoint.clone(),
                ws_rpc_url: client_config.ws_rpc_endpoint.clone(),
                blueprint_id,
                service_id: Some(service_id),
                tangle_contract: settings.tangle_contract,
                staking_contract: settings.staking_contract,
                status_registry_contract: settings.status_registry_contract,
                keystore_path: network.keystore_path()?.display().to_string(),
                data_dir,
                allow_unchecked_attestations,
                registration_mode: false,
                registration_capture_only: false,
                preferred_source: preferred_source_value,
                use_vm: use_vm_value,
                dry_run,
                shutdown_after: None,
            };
            run_blueprint(run_opts).await?;
        }
        ServiceCommands::List { network, json } => {
            let client = network.connect(0, None).await?;
            let services = list::services::list_services(&client).await?;
            list::services::print_services(&services, json);
        }
        ServiceCommands::Requests { network, json } => {
            let client = network.connect(0, None).await?;
            let requests = list::requests::list_requests(&client).await?;
            list::requests::print_requests(&requests, json);
        }
        ServiceCommands::Show {
            network,
            request_id,
        } => {
            let client = network.connect(0, None).await?;
            let request = client
                .get_service_request_info(request_id)
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            list::requests::print_request(&request);
        }
        ServiceCommands::SetPolicy {
            network,
            service_id,
            policy,
            json,
        } => {
            let ctx = upgrade_tx_ctx(&network)?;
            let tx = cargo_tangle::command::upgrade::set_service_policy(&ctx, service_id, policy)
                .await?;
            cargo_tangle::command::upgrade::print_simple_tx(
                &format!("set-policy({} -> {})", service_id, policy.as_str()),
                &tx,
                json,
            );
        }
        ServiceCommands::AckVersion {
            network,
            service_id,
            version_id,
            json,
        } => {
            let ctx = upgrade_tx_ctx(&network)?;
            let tx =
                cargo_tangle::command::upgrade::ack_version(&ctx, service_id, version_id).await?;
            cargo_tangle::command::upgrade::print_simple_tx(
                &format!("ack-version(service={service_id}, version={version_id})"),
                &tx,
                json,
            );
        }
        ServiceCommands::EffectiveVersion {
            network,
            service_id,
            blueprint_id,
            json,
        } => {
            let view = upgrade_view_ctx(&network, None)?;
            let v = cargo_tangle::command::upgrade::get_effective_version(
                &view,
                blueprint_id,
                service_id,
            )
            .await?;
            cargo_tangle::command::upgrade::print_effective_version(service_id, &v, json);
        }
        ServiceCommands::UpgradeStatus {
            network,
            service_id,
            blueprint_id,
            json,
        } => {
            let view = upgrade_view_ctx(&network, None)?;
            let policy =
                cargo_tangle::command::upgrade::get_service_policy(&view, service_id).await?;
            let acked =
                cargo_tangle::command::upgrade::get_service_acked_version_id(&view, service_id)
                    .await?;
            let active =
                cargo_tangle::command::upgrade::get_active_version_id(&view, blueprint_id).await?;
            let effective = cargo_tangle::command::upgrade::get_effective_version(
                &view,
                blueprint_id,
                service_id,
            )
            .await?;
            let versions =
                cargo_tangle::command::upgrade::list_versions(&view, blueprint_id).await?;
            let latest = versions
                .last()
                .map(|v| v.version_id)
                .unwrap_or(effective.version_id);
            let up_to_date = effective.version_id == latest;
            let status = cargo_tangle::command::upgrade::UpgradeStatus {
                service_id,
                policy,
                acked_version_id: acked,
                active_version_id: active,
                effective_version_id: effective.version_id,
                latest_version_id: latest,
                up_to_date,
            };
            cargo_tangle::command::upgrade::print_upgrade_status(&status, json);
        }
        ServiceCommands::Upgrades {
            service_id,
            manager_url,
            json,
        } => {
            let manager =
                cargo_tangle::command::upgrade_local::resolve_manager_url(manager_url.as_ref())?;
            let list =
                cargo_tangle::command::upgrade_local::list_upgrades(&manager, service_id).await?;
            cargo_tangle::command::upgrade_local::print_available(&list, json);
        }
        ServiceCommands::UpgradeLocal {
            service_id,
            version_id,
            dry_run,
            manager_url,
            json,
        } => {
            let manager =
                cargo_tangle::command::upgrade_local::resolve_manager_url(manager_url.as_ref())?;
            let result = cargo_tangle::command::upgrade_local::pin_version(
                &manager, service_id, version_id, dry_run,
            )
            .await?;
            cargo_tangle::command::upgrade_local::print_pin_result(service_id, &result, json);
        }
        ServiceCommands::UpgradeWhitelist {
            service_id,
            versions,
            manager_url,
            json,
        } => {
            let manager =
                cargo_tangle::command::upgrade_local::resolve_manager_url(manager_url.as_ref())?;
            let parsed = cargo_tangle::command::upgrade_local::parse_version_list(&versions)?;
            let result =
                cargo_tangle::command::upgrade_local::set_whitelist(&manager, service_id, parsed)
                    .await?;
            cargo_tangle::command::upgrade_local::print_whitelist_result(service_id, &result, json);
        }
        ServiceCommands::UpgradeSkip {
            service_id,
            version_id,
            reason,
            manager_url,
            json,
        } => {
            let manager =
                cargo_tangle::command::upgrade_local::resolve_manager_url(manager_url.as_ref())?;
            let result = cargo_tangle::command::upgrade_local::add_skip(
                &manager, service_id, version_id, reason,
            )
            .await?;
            cargo_tangle::command::upgrade_local::print_skip_result(service_id, &result, json);
        }
        ServiceCommands::UpgradeAuthz {
            service_id,
            manager_url,
            json,
        } => {
            let manager =
                cargo_tangle::command::upgrade_local::resolve_manager_url(manager_url.as_ref())?;
            let view =
                cargo_tangle::command::upgrade_local::show_authz(&manager, service_id).await?;
            cargo_tangle::command::upgrade_local::print_authz(&view, json);
        }
    }
    Ok(())
}

fn load_config_payload(config_file: Option<PathBuf>, config_hex: Option<String>) -> Result<Bytes> {
    if let Some(path) = config_file {
        let data = fs::read(&path)
            .with_context(|| format!("Failed to read config file {}", path.display()))?;
        return Ok(Bytes::from(data));
    }

    if let Some(hex_value) = config_hex {
        let trimmed = hex_value.trim_start_matches("0x");
        let bytes = hex::decode(trimmed).context("invalid config hex")?;
        return Ok(Bytes::from(bytes));
    }

    Ok(Bytes::new())
}
