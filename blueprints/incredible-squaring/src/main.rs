use color_eyre::Result;
use gadget_sdk::info;
use gadget_sdk::runners::tangle::TangleConfig;
use gadget_sdk::runners::BlueprintRunner;
use gadget_sdk::tangle_subxt::subxt::tx::Signer;
use incredible_squaring_blueprint as blueprint;

#[gadget_sdk::main(env)]
async fn main() {
    let x_square = blueprint::XsquareEventHandler::new(
        &env,
        blueprint::MyContext {
            config: env.clone(),
            call_id: None,
        },
    )
    .await?;

    let test_call_id = blueprint::TestCallIdEventHandler::new(
        &env,
        blueprint::MyContext {
            config: env.clone(),
            call_id: None,
        },
    )
    .await?;

    info!(
        "Starting the event watcher for {} ...",
        x_square.signer.account_id()
    );

    info!("~~~ Executing the incredible squaring blueprint ~~~");
    let tangle_config = TangleConfig::default();
    BlueprintRunner::new(tangle_config, env)
        .job(x_square)
        .job(test_call_id)
        .run()
        .await?;

    info!("Exiting...");
    Ok(())
}
