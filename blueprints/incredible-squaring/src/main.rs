use color_eyre::{eyre::eyre, Result};
use gadget_sdk::config::protocol::TangleInstanceSettings;
use gadget_sdk::info;
use gadget_sdk::runners::tangle::TangleConfig;
use gadget_sdk::runners::BlueprintRunner;
use gadget_sdk::tangle_subxt::subxt::tx::Signer;
use incredible_squaring_blueprint as blueprint;

#[gadget_sdk::main(env)]
async fn main() {
    let client = env.client().await.map_err(|e| eyre!(e))?;
    let signer = env.first_sr25519_signer().map_err(|e| eyre!(e))?;

    info!("Starting the event watcher for {} ...", signer.account_id());

    let tangle_settings = env.protocol_specific.tangle()?;
    let TangleInstanceSettings { service_id, .. } = tangle_settings;
    let x_square = blueprint::XsquareEventHandler {
        service_id: *service_id,
        context: blueprint::MyContext,
        client,
        signer,
    };

    info!("~~~ Executing the incredible squaring blueprint ~~~");
    let tangle_config = TangleConfig::default();
    BlueprintRunner::new(tangle_config, env)
        .job(x_square)
        .run()
        .await?;

    info!("Exiting...");
    Ok(())
}
