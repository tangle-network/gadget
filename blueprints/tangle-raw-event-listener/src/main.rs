use color_eyre::Result;
use gadget_sdk::info;
use gadget_sdk::runners::{tangle::TangleConfig, BlueprintRunner};
use gadget_sdk::tangle_subxt::subxt::tx::Signer;
use tangle_raw_event_listener_blueprint as blueprint;

#[gadget_sdk::main(env)]
async fn main() {
    let x_square = blueprint::RawEventHandler::new(&env, blueprint::MyContext).await?;
    let tangle_config = TangleConfig::default();

    info!(
        "~~~ Executing the incredible squaring blueprint for {:?} ~~~",
        x_square.signer.address()
    );
    BlueprintRunner::new(tangle_config, env)
        .job(x_square)
        .run()
        .await?;

    info!("Exiting...");
    Ok(())
}
