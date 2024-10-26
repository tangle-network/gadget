use color_eyre::Result;
use gadget_sdk::info;
use gadget_sdk::runners::tangle::TangleConfig;
use gadget_sdk::runners::BlueprintRunner;
use periodic_web_poller_blueprint as blueprint;

#[gadget_sdk::main(env)]
async fn main() {
    let web_poller = blueprint::WebPollerEventHandler {
        client: reqwest::Client::new(),
    };

    info!("~~~ Executing the periodic web poller ~~~");
    let tangle_config = TangleConfig::default();
    BlueprintRunner::new(tangle_config, env)
        .add_job(web_poller)
        .run()
        .await?;

    info!("Exiting...");
    Ok(())
}
