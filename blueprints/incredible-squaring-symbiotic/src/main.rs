use color_eyre::{eyre::eyre, Result};
use gadget_sdk::info;
use gadget_sdk::runners::symbiotic::SymbioticConfig;
use gadget_sdk::runners::BlueprintRunner;
use gadget_sdk::tangle_subxt::subxt::tx::Signer;
use incredible_squaring_blueprint_symbiotic as blueprint;

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
    let symb_config = SymbioticConfig {};
    BlueprintRunner::new(symb_config, env)
        .add_job(x_square)
        .run()
        .await?;

    info!("Exiting...");
    Ok(())
}
