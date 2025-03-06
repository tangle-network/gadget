use blueprint_sdk::logging::info;
use blueprint_sdk::macros::ext::tangle::tangle_subxt::subxt::tx::Signer;
use blueprint_sdk::runner::BlueprintRunner;
use blueprint_sdk::runner::tangle::config::TangleConfig;
use incredible_squaring_blueprint as blueprint;

#[tokio::main]
async fn main() -> Result<(), blueprint_sdk::Error> {
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
