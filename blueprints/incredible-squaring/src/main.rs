use color_eyre::{eyre::eyre, Result};
use gadget_sdk::config::{GadgetConfiguration};
use gadget_sdk::{
    tangle_subxt::tangle_testnet_runtime::api::{
        self,
        runtime_types::{sp_core::ecdsa, tangle_primitives::services},
    },
    tx,
    info
};
use incredible_squaring_blueprint as blueprint;
use gadget_sdk::job_runner::MultiJobRunner;
use gadget_sdk::keystore::sp_core_subxt::Pair;
use gadget_sdk::tangle_subxt::subxt::tx::Signer;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::PriceTargets;

#[gadget_sdk::main(env)]
async fn main() {
    let client = env.client().await.map_err(|e| eyre!(e))?;
    let signer = env.first_sr25519_signer().map_err(|e| eyre!(e))?;

    info!("Starting the event watcher for {} ...", signer.account_id());

    let x_square = blueprint::XsquareEventHandler {
        service_id: env.service_id.unwrap(),
        client: client.clone(),
        signer,
    };

    info!("~~~ Executing the incredible squaring blueprint ~~~");
    MultiJobRunner::new(&env)
        .with_job_context(env.clone())
        .with_registration(registration)
        .add_job(x_square)
        .run()
        .await?;

    info!("Exiting...");
    Ok(())
}

async fn registration(
    this: GadgetConfiguration<parking_lot::RawRwLock>,
) -> Result<(), gadget_sdk::Error> {
    let client = this
        .client()
        .await
        .map_err(|err| gadget_sdk::Error::Other(err.to_string()))?;
    let signer = this
        .first_sr25519_signer()
        .map_err(|err| gadget_sdk::Error::Other(err.to_string()))?;
    let ecdsa_pair = this
        .first_ecdsa_signer()
        .map_err(|err| gadget_sdk::Error::Other(err.to_string()))?;

    let xt = api::tx().services().register(
        this.blueprint_id,
        services::OperatorPreferences {
            key: ecdsa::Public(ecdsa_pair.signer().public().0),
            approval: services::ApprovalPrefrence::None,
            // TODO: Set the price targets
            price_targets: PriceTargets {
                cpu: 0,
                mem: 0,
                storage_hdd: 0,
                storage_ssd: 0,
                storage_nvme: 0,
            },
        },
        Default::default(),
    );

    // send the tx to the tangle and exit.
    let result = tx::tangle::send(&client, &signer, &xt).await?;
    info!("Registered operator with hash: {:?}", result);
    Ok(())
}
