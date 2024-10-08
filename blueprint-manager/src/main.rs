use blueprint_manager::config::BlueprintManagerConfig;
use blueprint_manager::run_blueprint_manager;
use blueprint_manager::sdk;
use blueprint_manager::sdk::utils::msg_to_error;
use gadget_io::GadgetConfig;
use sdk::entry;
use structopt::StructOpt;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let mut blueprint_manager_config = BlueprintManagerConfig::from_args();

    if let Some(data_dir) = blueprint_manager_config.data_dir.as_mut() {
        *data_dir = std::path::absolute(&data_dir)?;
    }

    entry::setup_blueprint_manager_logger(
        blueprint_manager_config.verbose,
        blueprint_manager_config.pretty,
        "gadget",
    )?;

    let Some(gadget_config) = blueprint_manager_config.gadget_config.as_ref() else {
        return Err(msg_to_error(
            "Gadget config file is required when running the blueprint manager in CLI mode"
                .to_string(),
        ));
    };

    let gadget_config_settings = std::fs::read_to_string(gadget_config)?;
    let gadget_config: GadgetConfig =
        toml::from_str(&gadget_config_settings).map_err(|err| msg_to_error(err.to_string()))?;

    // Allow CTRL-C to shutdown this CLI application instance
    let shutdown_signal = async move {
        let _ = tokio::signal::ctrl_c().await;
    };

    let handle =
        run_blueprint_manager(blueprint_manager_config, gadget_config, shutdown_signal).await?;
    handle.await?;

    Ok(())
}
