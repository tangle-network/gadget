use cargo_tangle::command::list;
use cargo_tangle::command::tangle::TangleClientArgs;
use clap::Subcommand;
use color_eyre::eyre::Result;

#[derive(Subcommand, Debug)]
pub(super) enum ListCommands {
    /// List all registered blueprints on the network.
    ///
    /// Shows blueprint IDs, names, owners, and operator requirements.
    Blueprints {
        #[command(flatten)]
        network: TangleClientArgs,
    },
    /// List all pending service requests awaiting operator approval.
    ///
    /// Shows requests that operators need to approve or reject.
    Requests {
        #[command(flatten)]
        network: TangleClientArgs,
    },
    /// List all active services on the network.
    ///
    /// Shows running service instances with their operators and status.
    Services {
        #[command(flatten)]
        network: TangleClientArgs,
    },
}

pub(super) async fn execute(command: ListCommands) -> Result<()> {
    match command {
        ListCommands::Blueprints { network } => {
            let client = network.connect(0, None).await?;
            let blueprints = list::blueprints::list_blueprints(&client).await?;
            list::blueprints::print_blueprints(&blueprints);
        }
        ListCommands::Requests { network } => {
            let client = network.connect(0, None).await?;
            let requests = list::requests::list_requests(&client).await?;
            list::requests::print_requests(&requests, false);
        }
        ListCommands::Services { network } => {
            let client = network.connect(0, None).await?;
            let services = list::services::list_services(&client).await?;
            list::services::print_services(&services, false);
        }
    }
    Ok(())
}
