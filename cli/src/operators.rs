use crate::staking_inputs::{parse_address_list, parse_commitments};
use crate::transaction_output::log_tx;
use alloy_primitives::U256;
use blueprint_client_tangle::DelegationMode;
use cargo_tangle::command::delegator;
use cargo_tangle::command::operator;
use cargo_tangle::command::service::{join_service, leave_service};
use cargo_tangle::command::tangle::{TangleClientArgs, parse_address};
use clap::Subcommand;
use color_eyre::eyre::{Result, eyre};
use serde_json::json;

#[derive(Subcommand, Debug)]
pub(super) enum OperatorCommands {
    /// Show operator heartbeat and status for a service.
    ///
    /// Displays the last heartbeat timestamp, status code, and health information.
    Status {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Blueprint ID the service belongs to.
        #[arg(long)]
        blueprint_id: u64,
        /// Service ID to check status for.
        #[arg(long)]
        service_id: u64,
        /// Operator address to query (defaults to your address).
        #[arg(long = "operator")]
        operator: Option<String>,
        /// Output as JSON instead of formatted display.
        #[arg(long)]
        json: bool,
    },
    /// Submit a heartbeat to signal operator liveness.
    ///
    /// Operators should submit heartbeats periodically to avoid being marked inactive.
    /// Status code 0 indicates healthy operation.
    #[command(visible_alias = "hb")]
    Heartbeat {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Blueprint ID the service belongs to.
        #[arg(long)]
        blueprint_id: u64,
        /// Service ID to submit heartbeat for.
        #[arg(long)]
        service_id: u64,
        /// Status code: 0 = healthy, non-zero = error code.
        #[arg(long, default_value_t = 0)]
        status_code: u8,
        /// Output transaction details as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Join a running dynamic service.
    ///
    /// For services with open membership, registers as an operator participant.
    /// Your stake exposure determines slashing risk for this service.
    Join {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Blueprint ID the service belongs to.
        #[arg(long)]
        blueprint_id: u64,
        /// Service ID to join.
        #[arg(long)]
        service_id: u64,
        /// Stake exposure in basis points (10000 = 100%).
        #[arg(long, default_value_t = 10_000)]
        exposure_bps: u16,
        /// Asset security commitment in format KIND:TOKEN:EXPOSURE_BPS.
        /// KIND: 0=ERC20, 1=Vault, 2=Native.
        /// TOKEN: Token/vault address (use 0x0 for native).
        /// EXPOSURE_BPS: Exposure in basis points (e.g., 5000 = 50%).
        /// Can be specified multiple times for multiple commitments.
        /// Example: --commitment 0:0x1234...abcd:5000
        #[arg(long, value_name = "KIND:TOKEN:EXPOSURE_BPS")]
        commitment: Vec<String>,
        /// Output transaction details as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Leave a dynamic service.
    ///
    /// Exits service participation. Your committed stake enters the unbonding period.
    Leave {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Blueprint ID the service belongs to.
        #[arg(long)]
        blueprint_id: u64,
        /// Service ID to leave.
        #[arg(long)]
        service_id: u64,
        /// Output transaction details as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Show operator staking status and stake amounts.
    ///
    /// Displays total stake, delegated amounts, and operator status.
    Staking {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Operator address to query (defaults to your address).
        #[arg(long = "operator")]
        operator: Option<String>,
        /// Output as JSON instead of formatted display.
        #[arg(long)]
        json: bool,
    },
    /// List all delegators who have staked with this operator.
    Delegators {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Operator address to query (defaults to your address).
        #[arg(long = "operator")]
        operator: Option<String>,
        /// Output as JSON instead of formatted table.
        #[arg(long)]
        json: bool,
    },
    /// Request to unstake operator bond.
    ///
    /// Initiates the unbonding period for operator stake.
    /// Use execute-unstake after the period ends.
    ScheduleUnstake {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Amount to unstake in wei (smallest token unit).
        #[arg(long)]
        amount: u128,
        /// Output transaction details as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Execute matured operator unstake requests.
    ///
    /// Completes unstaking for requests past the unbonding period.
    ExecuteUnstake {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Output transaction details as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Begin the process of leaving as an operator.
    ///
    /// Starts the exit period. You cannot accept new services while leaving.
    /// Use complete-leaving after the exit period ends.
    StartLeaving {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Output transaction details as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Complete the operator exit process.
    ///
    /// Finalizes leaving after the exit period. Removes operator status.
    CompleteLeaving {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Output transaction details as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Register as a new operator on the staking layer.
    ///
    /// Stakes the initial bond and enables operator status.
    /// For ERC20 bond tokens, you must approve() the staking contract first.
    Register {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Initial stake amount in wei (smallest token unit).
        #[arg(long)]
        amount: u128,
        /// Output transaction details as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Add more stake to your operator bond.
    ///
    /// Increases your total stake, improving your capacity for services.
    /// For ERC20 bond tokens, you must approve() the additional amount first.
    IncreaseStake {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Amount to add to stake in wei (smallest token unit).
        #[arg(long)]
        amount: u128,
        /// Output transaction details as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Get operator's delegation mode.
    ///
    /// Shows whether the operator accepts delegations and under what policy.
    GetDelegationMode {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Operator address (defaults to local account).
        #[arg(long = "operator")]
        operator: Option<String>,
        /// Emit JSON instead of human-readable output.
        #[arg(long)]
        json: bool,
    },
    /// Set delegation mode for the operator.
    ///
    /// Controls who can delegate to this operator:
    /// - disabled: Only operator can self-stake (default)
    /// - whitelist: Only approved addresses can delegate
    /// - open: Anyone can delegate
    SetDelegationMode {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Delegation mode: disabled, whitelist, or open.
        #[arg(long, value_enum)]
        mode: DelegationModeArg,
        /// Emit JSON instead of human-readable output.
        #[arg(long)]
        json: bool,
    },
    /// Update delegation whitelist.
    ///
    /// Add or remove addresses from the operator's delegation whitelist.
    /// Only applies when delegation mode is set to "whitelist".
    UpdateWhitelist {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Delegator addresses to update.
        #[arg(long = "delegator", required = true)]
        delegators: Vec<String>,
        /// Whether to approve (true) or revoke (false) the addresses.
        #[arg(long)]
        approved: bool,
        /// Emit JSON instead of human-readable output.
        #[arg(long)]
        json: bool,
    },
    /// Check if delegator can delegate to operator.
    CanDelegate {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Operator address.
        #[arg(long)]
        operator: String,
        /// Delegator address to check.
        #[arg(long)]
        delegator: String,
        /// Emit JSON instead of human-readable output.
        #[arg(long)]
        json: bool,
    },
    /// Schedule an exit from a dynamic service.
    ///
    /// Enters the operator into the exit queue. After the exit queue duration
    /// (default 7 days), use `execute-exit` to complete the exit.
    /// Requires the minimum commitment period to have passed since joining.
    ScheduleExit {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Service identifier.
        #[arg(long)]
        service_id: u64,
        /// Emit JSON transaction logs.
        #[arg(long)]
        json: bool,
    },
    /// Execute a previously scheduled exit from a service.
    ///
    /// Completes the exit after the exit queue duration has passed.
    /// Must be called after `schedule-exit` and waiting for the queue duration.
    ExecuteExit {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Service identifier.
        #[arg(long)]
        service_id: u64,
        /// Emit JSON transaction logs.
        #[arg(long)]
        json: bool,
    },
    /// Cancel a previously scheduled exit from a service.
    ///
    /// Cancels the exit and keeps the operator in the service.
    /// Can only be called before `execute-exit`.
    CancelExit {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Service identifier.
        #[arg(long)]
        service_id: u64,
        /// Emit JSON transaction logs.
        #[arg(long)]
        json: bool,
    },
}
/// Delegation mode argument for CLI.
#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub(super) enum DelegationModeArg {
    /// Only operator can self-stake (default).
    Disabled,
    /// Only approved addresses can delegate.
    Whitelist,
    /// Anyone can delegate.
    Open,
}

pub(super) async fn execute(command: OperatorCommands) -> Result<()> {
    match command {
        OperatorCommands::Status {
            network,
            blueprint_id,
            service_id,
            operator,
            json,
        } => {
            let client = network.connect(blueprint_id, Some(service_id)).await?;
            let operator_address = if let Some(value) = operator {
                parse_address(&value, "OPERATOR")?
            } else {
                client.account()
            };
            let status = client
                .operator_status(service_id, operator_address)
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            operator::print_status(&status, json);
        }
        OperatorCommands::Heartbeat {
            network,
            blueprint_id,
            service_id,
            status_code,
            json,
        } => {
            let config = network.client_config(blueprint_id, Some(service_id))?;
            let keystore = cargo_tangle::command::signer::load_keystore(network.keystore_path()?)?;
            let mut signing_key = cargo_tangle::command::signer::load_ecdsa_signing_key(&keystore)?;
            operator::submit_heartbeat(
                config.http_rpc_endpoint.as_str(),
                config.settings.status_registry_contract,
                &mut signing_key,
                service_id,
                blueprint_id,
                status_code,
                json,
            )
            .await?;
        }
        OperatorCommands::Join {
            network,
            blueprint_id,
            service_id,
            exposure_bps,
            commitment,
            json,
        } => {
            let client = network.connect(blueprint_id, Some(service_id)).await?;
            let tx = if commitment.is_empty() {
                client
                    .join_service(service_id, exposure_bps)
                    .await
                    .map_err(|e| eyre!(e.to_string()))?
            } else {
                let commitments = parse_commitments(&commitment)?;
                client
                    .join_service_with_commitments(service_id, exposure_bps, commitments)
                    .await
                    .map_err(|e| eyre!(e.to_string()))?
            };
            log_tx("Operator join", &tx, json);
        }
        OperatorCommands::Leave {
            network,
            blueprint_id,
            service_id,
            json,
        } => {
            let client = network.connect(blueprint_id, Some(service_id)).await?;
            let tx = client
                .leave_service(service_id)
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            log_tx("Operator leave", &tx, json);
        }
        OperatorCommands::Staking {
            network,
            operator,
            json,
        } => {
            let client = network.connect(0, None).await?;
            let operator_address = if let Some(value) = operator {
                parse_address(&value, "OPERATOR")?
            } else {
                client.account()
            };
            let is_registered = client
                .is_operator(operator_address)
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            let staking = client
                .get_restaking_metadata(operator_address)
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            let self_stake = client
                .get_operator_self_stake(operator_address)
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            let delegated_stake = client
                .get_operator_delegated_stake(operator_address)
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            let commission_bps = client
                .operator_commission_bps()
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            let current_round = client
                .restaking_round()
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            delegator::print_operator_staking(
                operator_address,
                &staking,
                is_registered,
                self_stake,
                delegated_stake,
                commission_bps,
                current_round,
                json,
            );
        }
        OperatorCommands::Delegators {
            network,
            operator,
            json,
        } => {
            let client = network.connect(0, None).await?;
            let operator_address = if let Some(value) = operator {
                parse_address(&value, "OPERATOR")?
            } else {
                client.account()
            };
            let delegators = client
                .get_operator_delegators(operator_address)
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            delegator::print_operator_delegators(operator_address, &delegators, json);
        }
        OperatorCommands::ScheduleUnstake {
            network,
            amount,
            json,
        } => {
            let client = network.connect(0, None).await?;
            let tx = client
                .schedule_operator_unstake(U256::from(amount))
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            log_tx("Operator schedule-unstake", &tx, json);
        }
        OperatorCommands::ExecuteUnstake { network, json } => {
            let client = network.connect(0, None).await?;
            let tx = client
                .execute_operator_unstake()
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            log_tx("Operator execute-unstake", &tx, json);
        }
        OperatorCommands::StartLeaving { network, json } => {
            let client = network.connect(0, None).await?;
            let tx = client
                .start_leaving()
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            log_tx("Operator start-leaving", &tx, json);
        }
        OperatorCommands::CompleteLeaving { network, json } => {
            let client = network.connect(0, None).await?;
            let tx = client
                .complete_leaving()
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            log_tx("Operator complete-leaving", &tx, json);
        }
        OperatorCommands::Register {
            network,
            amount,
            json,
        } => {
            let client = network.connect(0, None).await?;
            let tx = client
                .register_operator_restaking(U256::from(amount))
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            log_tx("Operator register", &tx, json);
        }
        OperatorCommands::IncreaseStake {
            network,
            amount,
            json,
        } => {
            let client = network.connect(0, None).await?;
            let tx = client
                .increase_stake(U256::from(amount))
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            log_tx("Operator increase-stake", &tx, json);
        }
        OperatorCommands::GetDelegationMode {
            network,
            operator,
            json,
        } => {
            let client = network.connect(0, None).await?;
            let operator_address = if let Some(value) = operator {
                parse_address(&value, "OPERATOR")?
            } else {
                client.account()
            };
            let mode = client
                .get_delegation_mode(operator_address)
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string(&serde_json::json!({
                        "operator": format!("{operator_address:?}"),
                        "delegation_mode": format!("{mode}")
                    }))?
                );
            } else {
                println!("Operator: {operator_address:?}");
                println!("Delegation Mode: {mode}");
            }
        }
        OperatorCommands::SetDelegationMode {
            network,
            mode,
            json,
        } => {
            let client = network.connect(0, None).await?;
            let delegation_mode = match mode {
                DelegationModeArg::Disabled => DelegationMode::Disabled,
                DelegationModeArg::Whitelist => DelegationMode::Whitelist,
                DelegationModeArg::Open => DelegationMode::Open,
            };
            let tx = client
                .set_delegation_mode(delegation_mode)
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            log_tx("Operator set-delegation-mode", &tx, json);
        }
        OperatorCommands::UpdateWhitelist {
            network,
            delegators,
            approved,
            json,
        } => {
            let client = network.connect(0, None).await?;
            let delegator_addresses = parse_address_list(&delegators, "DELEGATOR")?;
            let tx = client
                .set_delegation_whitelist(delegator_addresses, approved)
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            log_tx("Operator update-whitelist", &tx, json);
        }
        OperatorCommands::CanDelegate {
            network,
            operator,
            delegator,
            json,
        } => {
            let client = network.connect(0, None).await?;
            let operator_address = parse_address(&operator, "OPERATOR")?;
            let delegator_address = parse_address(&delegator, "DELEGATOR")?;
            let can_delegate = client
                .can_delegate(operator_address, delegator_address)
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string(&serde_json::json!({
                        "operator": format!("{operator_address:?}"),
                        "delegator": format!("{delegator_address:?}"),
                        "can_delegate": can_delegate
                    }))?
                );
            } else {
                println!("Operator: {operator_address:?}");
                println!("Delegator: {delegator_address:?}");
                println!("Can Delegate: {can_delegate}");
            }
        }
        OperatorCommands::ScheduleExit {
            network,
            service_id,
            json,
        } => {
            let client = network.connect(0, None).await?;
            let tx = client
                .schedule_exit(service_id)
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            log_tx("Operator schedule-exit", &tx, json);
        }
        OperatorCommands::ExecuteExit {
            network,
            service_id,
            json,
        } => {
            let client = network.connect(0, None).await?;
            let tx = client
                .execute_exit(service_id)
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            log_tx("Operator execute-exit", &tx, json);
        }
        OperatorCommands::CancelExit {
            network,
            service_id,
            json,
        } => {
            let client = network.connect(0, None).await?;
            let tx = client
                .cancel_exit(service_id)
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            log_tx("Operator cancel-exit", &tx, json);
        }
    }
    Ok(())
}
