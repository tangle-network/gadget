use color_eyre::Result;
use gadget_sdk::info;
use gadget_sdk::job_runner::MultiJobRunner;
use periodic_web_poller_blueprint as blueprint;

#[gadget_sdk::main]
async fn main() {
    let web_poller = blueprint::WebPollerEventHandler {
        client: reqwest::Client::new(),
    };

    info!("~~~ Executing the periodic web poller ~~~");
    MultiJobRunner::new(None)
        .with_job()
        .finish(web_poller)
        .run()
        .await?;

    info!("Exiting...");
    Ok(())
}
