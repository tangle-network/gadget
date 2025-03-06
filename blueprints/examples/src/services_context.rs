use blueprint_sdk::contexts::keystore::KeystoreContext;
use blueprint_sdk::contexts::tangle::TangleClientContext;
use blueprint_sdk::crypto::sp_core::SpSr25519;
use blueprint_sdk::event_listeners::core::InitializableEventHandler;
use blueprint_sdk::event_listeners::tangle::events::TangleEventListener;
use blueprint_sdk::event_listeners::tangle::services::{
    services_post_processor, services_pre_processor,
};
use blueprint_sdk::job;
use blueprint_sdk::keystore::backends::Backend;
use blueprint_sdk::logging::info;
use blueprint_sdk::macros::contexts::{ServicesContext, TangleClientContext};
use blueprint_sdk::macros::ext::clients::GadgetServicesClient;
use blueprint_sdk::runner::config::BlueprintEnvironment;
use blueprint_sdk::tangle_subxt::subxt::utils::AccountId32;
use blueprint_sdk::tangle_subxt::tangle_testnet_runtime::api::services::events::JobCalled;

#[derive(Clone, ServicesContext, TangleClientContext)]
pub struct ExampleServiceContext {
    #[config]
    sdk_config: BlueprintEnvironment,
    #[call_id]
    call_id: Option<u64>,
}

pub async fn constructor(
    env: BlueprintEnvironment,
) -> color_eyre::Result<impl InitializableEventHandler> {
    let signer = env
        .clone()
        .keystore()
        .first_local::<SpSr25519>()
        .map_err(|e| color_eyre::eyre::eyre!(e))?;

    info!("Starting the event watcher for {:?} ...", signer.0);
    HandleJobEventHandler::new(&env.clone(), ExampleServiceContext {
        sdk_config: env,
        call_id: None,
    })
    .await
    .map_err(|e| color_eyre::eyre::eyre!(e))
}

#[job(
    id = 3,
    params(job_details),
    event_listener(
        listener = TangleEventListener<ExampleServiceContext, JobCalled>,
        pre_processor = services_pre_processor,
        post_processor = services_post_processor
    ),
)]
pub async fn handle_job(
    context: ExampleServiceContext,
    job_details: Vec<u8>,
) -> Result<u64, blueprint_sdk::Error> {
    let client = context.tangle_client().await?;
    let blueprint_id = client.blueprint_id().await?;
    let block = client.now().await.unwrap();
    let blueprint_owner = client.current_blueprint_owner(block, blueprint_id).await?;
    let blueprint = client.current_blueprint_owner(block, blueprint_id).await?;
    let operators_and_percents = client
        .current_service_operators(block, blueprint_id)
        .await?;
    let operators = operators_and_percents
        .iter()
        .map(|(op, per)| op)
        .cloned()
        .collect::<Vec<AccountId32>>();
    Ok(1)
}
