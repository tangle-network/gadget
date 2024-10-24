use color_eyre::{eyre::eyre, Result};
use gadget_sdk::info;
use gadget_sdk::job_runner::MultiJobRunner;
use gadget_sdk::tangle_subxt::subxt::tx::Signer;
use incredible_squaring_blueprint as blueprint;

#[gadget_sdk::main(env)]
async fn main() {
    let client = env.client().await.map_err(|e| eyre!(e))?;
    let signer = env.first_sr25519_signer().map_err(|e| eyre!(e))?;

    info!("Starting the event watcher for {} ...", signer.account_id());

    let x_square = blueprint::XsquareEventHandler {
        service_id: env.service_id.unwrap(),
        context: blueprint::MyContext,
        client,
        signer,
    };

    info!("~~~ Executing the incredible squaring blueprint ~~~");
    MultiJobRunner::new(env).job(x_square).run().await?;

    info!("Exiting...");
    Ok(())
}
