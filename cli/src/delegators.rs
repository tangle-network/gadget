use crate::transaction_output::log_tx;
use alloy_primitives::{Address, U256};
use blueprint_client_tangle::BlueprintSelectionMode;
use cargo_tangle::command::tangle::{TangleClientArgs, parse_address};
use cargo_tangle::command::{delegator, operator};
use clap::Subcommand;
use color_eyre::eyre::{Result, ensure, eyre};
use serde_json::json;

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub(super) enum DelegationSelection {
    /// Delegation applies to all blueprints the operator supports.
    All,
    /// Delegation is pinned to specific blueprint IDs.
    Fixed,
}

#[derive(Subcommand, Debug)]
pub(super) enum DelegatorCommands {
    /// Show all staking positions for a delegator.
    ///
    /// Displays deposits, locks, active delegations, and pending requests.
    Positions {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Delegator address to query (defaults to your address).
        #[arg(long)]
        delegator: Option<String>,
        /// Token contract address (0x0 for native ETH/TNT).
        #[arg(long, default_value = "0x0000000000000000000000000000000000000000")]
        token: String,
        /// Output as JSON instead of formatted table.
        #[arg(long)]
        json: bool,
    },
    /// List active delegations from a delegator to operators.
    Delegations {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Delegator address to query (defaults to your address).
        #[arg(long)]
        delegator: Option<String>,
        /// Output as JSON instead of formatted table.
        #[arg(long)]
        json: bool,
    },
    /// List pending unstake requests waiting for unbonding period.
    PendingUnstakes {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Delegator address to query (defaults to your address).
        #[arg(long)]
        delegator: Option<String>,
        /// Output as JSON instead of formatted table.
        #[arg(long)]
        json: bool,
    },
    /// List pending withdrawal requests waiting for unbonding period.
    PendingWithdrawals {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Delegator address to query (defaults to your address).
        #[arg(long)]
        delegator: Option<String>,
        /// Output as JSON instead of formatted table.
        #[arg(long)]
        json: bool,
    },
    /// Check ERC20 token allowance for the staking contract.
    ///
    /// Shows how many tokens the contract can spend on your behalf.
    Allowance {
        #[command(flatten)]
        network: TangleClientArgs,
        /// ERC20 token contract address.
        #[arg(long)]
        token: String,
        /// Token owner address (defaults to your address).
        #[arg(long)]
        owner: Option<String>,
        /// Spender address (defaults to staking contract).
        #[arg(long)]
        spender: Option<String>,
        /// Output as JSON instead of formatted table.
        #[arg(long)]
        json: bool,
    },
    /// Check ERC20 token balance.
    Balance {
        #[command(flatten)]
        network: TangleClientArgs,
        /// ERC20 token contract address.
        #[arg(long)]
        token: String,
        /// Address to check balance for (defaults to your address).
        #[arg(long)]
        owner: Option<String>,
        /// Output as JSON instead of formatted table.
        #[arg(long)]
        json: bool,
    },
    /// Approve ERC20 tokens for staking.
    ///
    /// Required before depositing ERC20 tokens. Sets the allowance for
    /// the staking contract to transfer tokens on your behalf.
    Approve {
        #[command(flatten)]
        network: TangleClientArgs,
        /// ERC20 token contract address.
        #[arg(long)]
        token: String,
        /// Amount to approve in wei (smallest token unit).
        #[arg(long)]
        amount: u128,
        /// Spender address (defaults to staking contract).
        #[arg(long)]
        spender: Option<String>,
        /// Output transaction details as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Deposit tokens into the staking contract.
    ///
    /// Deposits tokens that can later be delegated to operators.
    /// For ERC20 tokens, you must approve() first.
    Deposit {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Token contract address (0x0 for native ETH/TNT).
        #[arg(long, default_value = "0x0000000000000000000000000000000000000000")]
        token: String,
        /// Amount to deposit in wei (smallest token unit).
        #[arg(long)]
        amount: u128,
        /// Output transaction details as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Delegate deposited tokens to an operator.
    ///
    /// Assigns your stake to an operator who provides economic security for services.
    /// You earn rewards when the operator participates in services.
    Delegate {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Operator address to delegate stake to.
        #[arg(long)]
        operator: String,
        /// Amount to delegate in wei (smallest token unit).
        #[arg(long)]
        amount: u128,
        /// Token contract address (0x0 for native ETH/TNT).
        #[arg(long, default_value = "0x0000000000000000000000000000000000000000")]
        token: String,
        /// Blueprint selection: all (any blueprint) or fixed (specific blueprints).
        #[arg(long, value_enum, default_value = "all")]
        selection: DelegationSelection,
        /// Blueprint IDs for fixed selection (requires --selection=fixed).
        #[arg(long = "blueprint-id")]
        blueprint_ids: Vec<u64>,
        /// Delegate from existing deposit balance instead of new deposit.
        #[arg(long)]
        from_deposit: bool,
        /// Output transaction details as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Request to undelegate stake from an operator.
    ///
    /// Initiates the unbonding period. Use execute-unstake after the period ends.
    Undelegate {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Operator address to undelegate from.
        #[arg(long)]
        operator: String,
        /// Amount to undelegate in wei (smallest token unit).
        #[arg(long)]
        amount: u128,
        /// Token contract address (0x0 for native ETH/TNT).
        #[arg(long, default_value = "0x0000000000000000000000000000000000000000")]
        token: String,
        /// Output transaction details as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Execute all matured unstake requests.
    ///
    /// Completes undelegation for requests past the unbonding period.
    /// Tokens move back to your deposit balance.
    ExecuteUnstake {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Output transaction details as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Execute a specific unstake and withdraw in one transaction.
    ///
    /// Completes undelegation and immediately withdraws to your wallet.
    ExecuteUnstakeWithdraw {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Operator address from the original delegation.
        #[arg(long)]
        operator: String,
        /// Token contract address (0x0 for native ETH/TNT).
        #[arg(long, default_value = "0x0000000000000000000000000000000000000000")]
        token: String,
        /// Share amount from your pending unstake.
        #[arg(long)]
        shares: u128,
        /// Round number from the pending unstake request.
        #[arg(long)]
        requested_round: u64,
        /// Recipient address (defaults to your address).
        #[arg(long)]
        receiver: Option<String>,
        /// Output transaction details as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Request to withdraw deposited tokens.
    ///
    /// Initiates the unbonding period for non-delegated deposits.
    /// Use execute-withdraw after the period ends.
    ScheduleWithdraw {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Token contract address (0x0 for native ETH/TNT).
        #[arg(long, default_value = "0x0000000000000000000000000000000000000000")]
        token: String,
        /// Amount to withdraw in wei (smallest token unit).
        #[arg(long)]
        amount: u128,
        /// Output transaction details as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Execute all matured withdrawal requests.
    ///
    /// Transfers tokens back to your wallet for requests past the unbonding period.
    ExecuteWithdraw {
        #[command(flatten)]
        network: TangleClientArgs,
        /// Output transaction details as JSON.
        #[arg(long)]
        json: bool,
    },
}

pub(super) async fn execute(command: DelegatorCommands) -> Result<()> {
    match command {
        DelegatorCommands::Positions {
            network,
            delegator,
            token,
            json,
        } => {
            let client = network.connect(0, None).await?;
            let delegator_address = if let Some(value) = delegator {
                parse_address(&value, "DELEGATOR")?
            } else {
                client.account()
            };
            let token_address = parse_address(&token, "TOKEN")?;
            let deposit = client
                .get_deposit_info(delegator_address, token_address)
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            let locks = client
                .get_locks(delegator_address, token_address)
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            let delegations = client
                .get_delegations_with_blueprints(delegator_address)
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            let unstakes = client
                .get_pending_unstakes(delegator_address)
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            let withdrawals = client
                .get_pending_withdrawals(delegator_address)
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            delegator::print_positions(
                delegator_address,
                token_address,
                &deposit,
                &locks,
                &delegations,
                &unstakes,
                &withdrawals,
                json,
            );
        }
        DelegatorCommands::Delegations {
            network,
            delegator,
            json,
        } => {
            let client = network.connect(0, None).await?;
            let delegator_address = if let Some(value) = delegator {
                parse_address(&value, "DELEGATOR")?
            } else {
                client.account()
            };
            let delegations = client
                .get_delegations_with_blueprints(delegator_address)
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            delegator::print_delegations(delegator_address, &delegations, json);
        }
        DelegatorCommands::PendingUnstakes {
            network,
            delegator,
            json,
        } => {
            let client = network.connect(0, None).await?;
            let delegator_address = if let Some(value) = delegator {
                parse_address(&value, "DELEGATOR")?
            } else {
                client.account()
            };
            let unstakes = client
                .get_pending_unstakes(delegator_address)
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            delegator::print_pending_unstakes(delegator_address, &unstakes, json);
        }
        DelegatorCommands::PendingWithdrawals {
            network,
            delegator,
            json,
        } => {
            let client = network.connect(0, None).await?;
            let delegator_address = if let Some(value) = delegator {
                parse_address(&value, "DELEGATOR")?
            } else {
                client.account()
            };
            let withdrawals = client
                .get_pending_withdrawals(delegator_address)
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            delegator::print_pending_withdrawals(delegator_address, &withdrawals, json);
        }
        DelegatorCommands::Allowance {
            network,
            token,
            owner,
            spender,
            json,
        } => {
            let client = network.connect(0, None).await?;
            let token_address = parse_address(&token, "TOKEN")?;
            ensure!(
                token_address != Address::ZERO,
                "Token address must be non-zero for ERC20 allowance"
            );
            let owner_address = if let Some(value) = owner {
                parse_address(&value, "OWNER")?
            } else {
                client.account()
            };
            let spender_address = if let Some(value) = spender {
                parse_address(&value, "SPENDER")?
            } else {
                client.config.settings.staking_contract
            };
            let allowance = client
                .erc20_allowance(token_address, owner_address, spender_address)
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            delegator::print_erc20_allowance(
                owner_address,
                spender_address,
                token_address,
                allowance,
                json,
            );
        }
        DelegatorCommands::Balance {
            network,
            token,
            owner,
            json,
        } => {
            let client = network.connect(0, None).await?;
            let token_address = parse_address(&token, "TOKEN")?;
            ensure!(
                token_address != Address::ZERO,
                "Token address must be non-zero for ERC20 balance"
            );
            let owner_address = if let Some(value) = owner {
                parse_address(&value, "OWNER")?
            } else {
                client.account()
            };
            let balance = client
                .erc20_balance(token_address, owner_address)
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            delegator::print_erc20_balance(owner_address, token_address, balance, json);
        }
        DelegatorCommands::Approve {
            network,
            token,
            amount,
            spender,
            json,
        } => {
            let client = network.connect(0, None).await?;
            let token_address = parse_address(&token, "TOKEN")?;
            ensure!(
                token_address != Address::ZERO,
                "Token address must be non-zero for ERC20 approvals"
            );
            let spender_address = if let Some(value) = spender {
                parse_address(&value, "SPENDER")?
            } else {
                client.config.settings.staking_contract
            };
            let tx = client
                .erc20_approve(token_address, spender_address, U256::from(amount))
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            log_tx("Delegator approve", &tx, json);
        }
        DelegatorCommands::Deposit {
            network,
            token,
            amount,
            json,
        } => {
            let client = network.connect(0, None).await?;
            let token_address = parse_address(&token, "TOKEN")?;
            let tx = if token_address == Address::ZERO {
                client
                    .deposit_native(U256::from(amount))
                    .await
                    .map_err(|e| eyre!(e.to_string()))?
            } else {
                client
                    .deposit_erc20(token_address, U256::from(amount))
                    .await
                    .map_err(|e| eyre!(e.to_string()))?
            };
            log_tx("Delegator deposit", &tx, json);
        }
        DelegatorCommands::Delegate {
            network,
            operator,
            amount,
            token,
            selection,
            blueprint_ids,
            from_deposit,
            json,
        } => {
            let client = network.connect(0, None).await?;
            let operator_address = parse_address(&operator, "OPERATOR")?;
            let token_address = parse_address(&token, "TOKEN")?;
            let selection_mode = match selection {
                DelegationSelection::All => BlueprintSelectionMode::All,
                DelegationSelection::Fixed => BlueprintSelectionMode::Fixed,
            };
            if matches!(selection_mode, BlueprintSelectionMode::Fixed) && blueprint_ids.is_empty() {
                return Err(eyre!(
                    "Fixed selection requires at least one --blueprint-id"
                ));
            }
            let tx = if from_deposit {
                client
                    .delegate_with_options(
                        operator_address,
                        token_address,
                        U256::from(amount),
                        selection_mode,
                        blueprint_ids,
                    )
                    .await
            } else {
                client
                    .deposit_and_delegate_with_options(
                        operator_address,
                        token_address,
                        U256::from(amount),
                        selection_mode,
                        blueprint_ids,
                    )
                    .await
            }
            .map_err(|e| eyre!(e.to_string()))?;
            log_tx("Delegator delegate", &tx, json);
        }
        DelegatorCommands::Undelegate {
            network,
            operator,
            amount,
            token,
            json,
        } => {
            let client = network.connect(0, None).await?;
            let operator_address = parse_address(&operator, "OPERATOR")?;
            let token_address = parse_address(&token, "TOKEN")?;
            let tx = client
                .schedule_delegator_unstake(operator_address, token_address, U256::from(amount))
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            log_tx("Delegator undelegate", &tx, json);
        }
        DelegatorCommands::ExecuteUnstake { network, json } => {
            let client = network.connect(0, None).await?;
            let tx = client
                .execute_delegator_unstake()
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            log_tx("Delegator execute-unstake", &tx, json);
        }
        DelegatorCommands::ExecuteUnstakeWithdraw {
            network,
            operator,
            token,
            shares,
            requested_round,
            receiver,
            json,
        } => {
            let client = network.connect(0, None).await?;
            let operator_address = parse_address(&operator, "OPERATOR")?;
            let token_address = parse_address(&token, "TOKEN")?;
            let receiver = if let Some(value) = receiver {
                parse_address(&value, "RECEIVER")?
            } else {
                client.account()
            };
            let tx = client
                .execute_delegator_unstake_and_withdraw(
                    operator_address,
                    token_address,
                    U256::from(shares),
                    requested_round,
                    receiver,
                )
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            log_tx("Delegator execute-unstake-withdraw", &tx, json);
        }
        DelegatorCommands::ScheduleWithdraw {
            network,
            token,
            amount,
            json,
        } => {
            let client = network.connect(0, None).await?;
            let token_address = parse_address(&token, "TOKEN")?;
            let tx = client
                .schedule_withdraw(token_address, U256::from(amount))
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            log_tx("Delegator schedule-withdraw", &tx, json);
        }
        DelegatorCommands::ExecuteWithdraw { network, json } => {
            let client = network.connect(0, None).await?;
            let tx = client
                .execute_withdraw()
                .await
                .map_err(|e| eyre!(e.to_string()))?;
            log_tx("Delegator execute-withdraw", &tx, json);
        }
    }
    Ok(())
}
