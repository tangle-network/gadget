use color_eyre::{eyre::eyre, Result};
use gadget_sdk::config::{ContextConfig, GadgetConfiguration, StdGadgetConfiguration};
use gadget_sdk::{
    tangle_subxt::tangle_testnet_runtime::api::{
        self,
        runtime_types::{sp_core::ecdsa, tangle_primitives::services},
    },
    tx,
    info
};
use incredible_squaring_blueprint as blueprint;
use structopt::StructOpt;
use gadget_sdk::events_watcher::InitializableEventHandler;
use gadget_sdk::job_runner::MultiJobRunner;
use gadget_sdk::keystore::sp_core_subxt::Pair;
use gadget_sdk::run::GadgetRunner;
use gadget_sdk::tangle_subxt::subxt::tx::Signer;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::PriceTargets;
struct TangleGadgetRunner {
    env: GadgetConfiguration<parking_lot::RawRwLock>,
}

#[async_trait::async_trait]
impl GadgetRunner for TangleGadgetRunner {
    type Error = color_eyre::eyre::Report;

    fn config(&self) -> &StdGadgetConfiguration {
        todo!()
    }

    async fn register(&mut self) -> Result<()> {
        // TODO: Use the function in blueprint-test-utils
        if self.env.test_mode {
            info!("Skipping registration in test mode");
            return Ok(());
        }

        let client = self.env.client().await.map_err(|e| eyre!(e))?;
        let signer = self
            .env
            .first_sr25519_signer()
            .map_err(|e| eyre!(e))
            .map_err(|e| eyre!(e))?;
        let ecdsa_pair = self.env.first_ecdsa_signer().map_err(|e| eyre!(e))?;

        let xt = api::tx().services().register(
            self.env.blueprint_id,
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

    async fn benchmark(&self) -> std::result::Result<(), Self::Error> {
        todo!()
    }

    async fn run(&mut self) -> Result<()> {
        let client = self.env.client().await.map_err(|e| eyre!(e))?;
        let signer = self.env.first_sr25519_signer().map_err(|e| eyre!(e))?;

        info!("Starting the event watcher for {} ...", signer.account_id());

        let x_square = blueprint::XsquareEventHandler {
            service_id: self.env.service_id.unwrap(),
            client: client.clone(),
            signer,
        };

        let finished_rx = x_square
            .init_event_handler()
            .await
            .expect("Event Listener init already called");
        let res = finished_rx.await;
        gadget_sdk::error!("Event handler finished with {res:?}");

        Ok(())
    }
}

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
    MultiJobRunner::default().with_job(x_square).run().await?;

    info!("Exiting...");
    Ok(())
}
