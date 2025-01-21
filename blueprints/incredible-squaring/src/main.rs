use blueprint_sdk::logging::info;
use blueprint_sdk::macros as gadget_macros;
use blueprint_sdk::macros::ext::tangle::tangle_subxt::subxt::tx::Signer;
use blueprint_sdk::runners::core::runner::BlueprintRunner;
use blueprint_sdk::runners::tangle::tangle::TangleConfig;
use incredible_squaring_blueprint as blueprint;

#[blueprint_sdk::main(env)]
async fn main() {
    let context = blueprint::MyContext {
        env: env.clone(),
        call_id: None,
    };

    let x_square = blueprint::XsquareEventHandler::new(&env, context).await?;

    info!(
        "Starting the event watcher for {} ...",
        x_square.signer.account_id()
    );

    info!("~~~ Executing the incredible squaring blueprint ~~~");
    let tangle_config = TangleConfig::default();
    BlueprintRunner::new(tangle_config, env)
        .job(x_square)
        .run()
        .await?;

    info!("Exiting...");
    Ok(())
}
