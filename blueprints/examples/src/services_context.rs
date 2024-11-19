use gadget_sdk::ctx::TangleClientContext;
use gadget_sdk::event_listener::tangle::jobs::{services_post_processor, services_pre_processor};
use gadget_sdk::event_listener::tangle::TangleEventListener;
use gadget_sdk::event_utils::InitializableEventHandler;
use gadget_sdk::job;
use gadget_sdk::subxt_core::utils::AccountId32;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::services::events::JobCalled;
use gadget_sdk::{config::StdGadgetConfiguration, ctx::ServicesContext};

#[derive(Clone, ServicesContext, TangleClientContext)]
pub struct ExampleServiceContext {
    #[config]
    sdk_config: StdGadgetConfiguration,
}

pub async fn constructor(
    env: StdGadgetConfiguration,
) -> color_eyre::Result<impl InitializableEventHandler> {
    use gadget_sdk::subxt_core::tx::signer::Signer;

    let signer = env
        .clone()
        .first_sr25519_signer()
        .map_err(|e| color_eyre::eyre::eyre!(e))?;

    gadget_sdk::info!("Starting the event watcher for {} ...", signer.account_id());
    HandleJobEventHandler::new(&env.clone(), ExampleServiceContext { sdk_config: env })
        .await
        .map_err(|e| color_eyre::eyre::eyre!(e))
}

#[job(
    id = 2,
    params(job_details),
    event_listener(
        listener = TangleEventListener<ExampleServiceContext, JobCalled>,
        pre_processor = services_pre_processor,
        post_processor = services_post_processor
    ),
)]
pub async fn handle_job(
    job_details: Vec<u8>,
    context: ExampleServiceContext,
) -> Result<u64, gadget_sdk::Error> {
    let client = context.tangle_client().await.unwrap();
    let blueprint_owner = context.current_blueprint_owner(&client).await.unwrap();
    let blueprint = context.current_blueprint(&client).await.unwrap();
    let operators_and_percents = context.current_service_operators(&client).await.unwrap();
    let operators = operators_and_percents
        .iter()
        .map(|(op, per)| op)
        .cloned()
        .collect::<Vec<AccountId32>>();
    let restaking_delegations = context
        .operator_delegations(&client, operators.clone())
        .await
        .unwrap();
    let operators_metadata = context
        .operators_metadata(&client, operators)
        .await
        .unwrap();
    Ok(0)
}
