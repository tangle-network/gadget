use blueprint_manager::config::BlueprintManagerConfig;
use blueprint_manager::run_blueprint_manager;
use blueprint_manager::utils;
use gadget_common::gadget_io;
use gadget_io::GadgetConfig;
use shell_sdk::entry;
use structopt::StructOpt;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    //color_eyre::install()?;
    let blueprint_manager_config = &BlueprintManagerConfig::from_args();
    entry::setup_shell_logger(
        blueprint_manager_config.verbose,
        blueprint_manager_config.pretty,
        "gadget",
    )?;
    let gadget_config_settings =
        std::fs::read_to_string(blueprint_manager_config.gadget_config.clone())?;
    let gadget_config: GadgetConfig = toml::from_str(&gadget_config_settings)
        .map_err(|err| utils::msg_to_error(err.to_string()))?;

    run_blueprint_manager(blueprint_manager_config, gadget_config).await
}
