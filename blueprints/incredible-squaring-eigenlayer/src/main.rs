use alloy_network::EthereumWallet;
use alloy_provider::ProviderBuilder;
use alloy_signer_local::PrivateKeySigner;
use color_eyre::Result;
use gadget_sdk::{
    events_watcher::evm::DefaultNodeConfig,
    info,
    runners::{eigenlayer::EigenlayerConfig, BlueprintRunner},
};
use incredible_squaring_blueprint_eigenlayer::{
    constants::{AGGREGATOR_PRIVATE_KEY, TASK_MANAGER_ADDRESS},
    contexts::{aggregator::AggregatorContext, client::AggregatorClient},
    jobs::{
        compute_x_square::XsquareEigenEventHandler, initialize_task::InitializeBlsTaskEventHandler,
    },
    IncredibleSquaringTaskManager,
};

#[gadget_sdk::main(env)]
async fn main() {
    // Get the ECDSA key from the private key seed using alloy
    let signer: PrivateKeySigner = AGGREGATOR_PRIVATE_KEY
        .parse()
        .expect("failed to generate wallet ");
    let wallet = EthereumWallet::from(signer);
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet.clone())
        .on_http(env.http_rpc_endpoint.parse()?);
    info!("Task Manager Address: {:?}", *TASK_MANAGER_ADDRESS);
    let contract = IncredibleSquaringTaskManager::IncredibleSquaringTaskManagerInstance::new(
        *TASK_MANAGER_ADDRESS,
        provider,
    );

    info!("Protocol: Eigenlayer");
    info!(
        "Registry Coordinator Address: {:?}",
        env.protocol_specific
            .eigenlayer()?
            .registry_coordinator_address
    );
    info!(
        "Operator State Retriever Address: {:?}",
        env.protocol_specific
            .eigenlayer()?
            .operator_state_retriever_address
    );
    info!(
        "Delegation Manager Address: {:?}",
        env.protocol_specific
            .eigenlayer()?
            .delegation_manager_address
    );
    info!(
        "Strategy Manager Address: {:?}",
        env.protocol_specific.eigenlayer()?.strategy_manager_address
    );
    info!(
        "AVS Directory Address: {:?}",
        env.protocol_specific.eigenlayer()?.avs_directory_address
    );

    let server_address = format!("{}:{}", env.bind_addr, 8081);
    let aggregator_client = AggregatorClient::new(&server_address)?;
    let x_square_eigen = XsquareEigenEventHandler::<DefaultNodeConfig> {
        ctx: aggregator_client,
        contract: contract.clone().into(),
    };

    let aggregator_context = AggregatorContext::new(
        server_address,
        *TASK_MANAGER_ADDRESS,
        env.http_rpc_endpoint.clone(),
        wallet,
        env.clone(),
    )
    .await
    .unwrap();

    let initialize_task = InitializeBlsTaskEventHandler::<DefaultNodeConfig> {
        ctx: aggregator_context.clone(),
        contract: contract.clone().into(),
    };

    // let (handle, aggregator_shutdown_tx) =
    // aggregator_context.start(env.ws_rpc_endpoint.clone());

    info!("~~~ Executing the incredible squaring blueprint ~~~");
    let eigen_config = EigenlayerConfig {};
    BlueprintRunner::new(eigen_config, env)
        .job(x_square_eigen)
        .job(initialize_task)
        .background_service(Box::new(aggregator_context))
        .run()
        .await?;

    info!("Exiting...");
    Ok(())
}
