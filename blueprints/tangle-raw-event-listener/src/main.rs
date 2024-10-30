use color_eyre::{eyre::eyre, Result};
use gadget_sdk::info;
use gadget_sdk::runners::{tangle::TangleConfig, BlueprintRunner};
use gadget_sdk::tangle_subxt::subxt::tx::Signer;
use tangle_raw_event_listener_blueprint as blueprint;

#[gadget_sdk::main(env)]
async fn main() {
    let client = env.client().await.map_err(|e| eyre!(e))?;
    let signer = env.first_sr25519_signer().map_err(|e| eyre!(e))?;

    info!("Starting the event watcher for {} ...", signer.account_id());

    let x_square = blueprint::RawEventHandler {
        service_id: env.service_id().expect("No service ID found"),
        context: blueprint::MyContext,
        client,
        signer,
    };

    let tangle_config = TangleConfig::default();

    info!("~~~ Executing the incredible squaring blueprint ~~~");
    BlueprintRunner::new(tangle_config, env)
        .job(x_square)
        .run()
        .await?;

    info!("Exiting...");
    Ok(())
}
