use blueprint::examples::*;
use blueprint_examples as blueprint;
use gadget_sdk::info;
use gadget_sdk::runners::{tangle::TangleConfig, BlueprintRunner};

#[gadget_sdk::main(env)]
async fn main() {
    info!("~~~ Executing the incredible squaring blueprint ~~~");
    BlueprintRunner::new(TangleConfig::default(), env.clone())
        .job(raw_tangle_events::constructor(env.clone()).await?)
        .job(periodic_web_poller::constructor())
        .run()
        .await?;

    info!("Exiting...");
    Ok(())
}
